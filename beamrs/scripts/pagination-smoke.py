#!/usr/bin/env python3
"""Exercise cursor pagination on every ray feed of a running BeamRS server."""

import json
import sys
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080"
USERNAME = "PaginationSmokeUser"


def request(path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    headers = {"Content-Type": "application/json"} if data else {}
    req = Request(
        BASE + path, data=data, headers=headers, method="POST" if data else "GET"
    )
    with urlopen(req, timeout=30) as response:
        return response.status, json.load(response)


def walk(path, page_size):
    seen = []
    cursor = None
    while True:
        query = {"limit": page_size}
        if cursor is not None:
            query["cursor"] = cursor
        status, page = request(path + "?" + urlencode(query))
        assert status == 200, f"{path}: status {status}"
        assert set(page) == {"rays", "next_cursor"}, f"{path}: keys {sorted(page)}"
        assert len(page["rays"]) <= page_size, f"{path}: page exceeds limit"
        seen.extend(ray["id"] for ray in page["rays"])
        cursor = page["next_cursor"]
        if cursor is None:
            return seen
        assert isinstance(cursor, str) and cursor, f"{path}: invalid cursor"
        assert len(seen) < 10_000, f"{path}: pagination did not terminate"


def main():
    status, user = request("/api/users", {"username": USERNAME})
    assert status == 200, f"create user: status {status}"

    _, frequencies = request("/api/frequencies")
    frequency_id = frequencies[0]["id"]

    created = []
    for index in range(3):
        status, ray = request(
            "/api/rays",
            {
                "frequency_id": frequency_id,
                "user_id": user["id"],
                "text": f"Pagination smoke ray {index}",
            },
        )
        assert status == 201, f"create ray: status {status}"
        created.append(ray["id"])
    for ray_id in created[:2]:
        status, _ = request("/api/prisms", {"user_id": user["id"], "ray_id": ray_id})
        assert status == 200, f"prism: status {status}"

    encoded = quote(USERNAME, safe="")
    feeds = {
        f"/api/frequencies/{frequency_id}/rays": None,
        f"/api/users/{encoded}/rays": created,
        f"/api/users/{encoded}/prisms": created[:2],
    }
    for path, expected_subset in feeds.items():
        _, unpaged = request(path + "?limit=100")
        assert unpaged["next_cursor"] is None, f"{path}: smoke dataset exceeds one page"
        full = [ray["id"] for ray in unpaged["rays"]]
        paged = walk(path, 1)
        assert paged == full, f"{path}: paged order {paged} != full order {full}"
        assert len(paged) == len(set(paged)), f"{path}: duplicate rays across pages"
        if expected_subset is not None:
            assert set(expected_subset) <= set(paged), f"{path}: missing created rays"
            assert paged[: len(expected_subset)] == sorted(
                expected_subset, reverse=True
            ), f"{path}: newest-first order violated"
        print(f"{path}: {len(paged)} rays paged one at a time")

    _, authored = request(f"/api/users/{encoded}/rays?limit=100")
    by_id = {ray["id"]: ray for ray in authored["rays"]}
    for ray_id in created:
        ray = by_id[ray_id]
        expected_prismers = [USERNAME] if ray_id in created[:2] else []
        assert ray["user_name"] == USERNAME, f"ray {ray_id}: author {ray['user_name']}"
        assert ray["users_prismed"] == expected_prismers, (
            f"ray {ray_id}: prismers {ray['users_prismed']}"
        )
        assert ray["prism_count"] == len(expected_prismers), (
            f"ray {ray_id}: prism count {ray['prism_count']}"
        )
    print("Feed rays carry author and prism details")

    for bad in ("limit=0", "limit=101", "cursor=garbage", "cursor=9223372036854775807:1"):
        try:
            request(f"/api/frequencies/{frequency_id}/rays?{bad}")
        except Exception as error:
            assert getattr(error, "code", None) == 400, f"{bad}: {error}"
        else:
            raise AssertionError(f"{bad}: expected HTTP 400")
    print("Pagination smoke test passed")


if __name__ == "__main__":
    main()
