const storageKey = "beamrs.identity";

function randomAnonName() {
  const values = new Uint32Array(1);
  crypto.getRandomValues(values);
  return `Anon${(values[0] % 900000) + 100000}`;
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    ...options,
    headers: { "Content-Type": "application/json", ...(options.headers || {}) },
  });
  const contentType = response.headers.get("content-type") || "";
  const body = response.status === 204
    ? null
    : contentType.includes("application/json")
      ? await response.json()
      : { error: await response.text() };
  if (!response.ok) {
    throw new Error(body?.error || `Request failed with status ${response.status}`);
  }
  return body;
}

async function loadIdentity() {
  let saved = null;
  try {
    saved = JSON.parse(localStorage.getItem(storageKey));
  } catch {
    localStorage.removeItem(storageKey);
  }
  const username = saved?.username || randomAnonName();
  const user = await api("/api/users", {
    method: "POST",
    body: JSON.stringify({ username }),
  });
  localStorage.setItem(storageKey, JSON.stringify(user));
  return user;
}

function showToast(message, isError = false) {
  const toast = document.querySelector("#toast");
  if (!toast) return;
  toast.textContent = message;
  toast.classList.toggle("error", isError);
  toast.classList.add("visible");
  window.setTimeout(() => toast.classList.remove("visible"), 3500);
}

function setStatus(form, message, isError = false) {
  const status = form.querySelector(".form-status");
  if (!status) return;
  status.textContent = message;
  status.classList.toggle("error", isError);
}

function wireIdentity(user) {
  const username = document.querySelector("#current-username");
  const profileLink = document.querySelector("#my-profile-link");
  if (username) username.textContent = user.username;
  if (profileLink) profileLink.href = `/user/${encodeURIComponent(user.username)}`;

  document.querySelectorAll(".prism-button").forEach((button) => {
    const prismUsers = (button.dataset.prismUsers || "").split(",").filter(Boolean);
    const prismed = prismUsers.includes(user.username);
    button.setAttribute("aria-pressed", String(prismed));
    button.classList.toggle("active", prismed);
    button.querySelector(".prism-icon").textContent = prismed ? "◆" : "◇";
  });

  const usernameInput = document.querySelector("#username-input");
  if (usernameInput) usernameInput.value = user.username;
}

function wireFrequencyForm() {
  const form = document.querySelector("#frequency-form");
  if (!form) return;
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const name = new FormData(form).get("name");
    setStatus(form, "Creating frequency…");
    try {
      const frequency = await api("/api/frequencies", {
        method: "POST",
        body: JSON.stringify({ name }),
      });
      window.location.assign(`/frequency/${frequency.id}`);
    } catch (error) {
      setStatus(form, error.message, true);
    }
  });
}

function wireRayForm(user) {
  const form = document.querySelector("#ray-form");
  if (!form) return;
  const textarea = form.querySelector("#ray-text");
  const counter = form.querySelector("#ray-character-count");
  textarea.addEventListener("input", () => {
    counter.textContent = String([...textarea.value].length);
  });
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    setStatus(form, "Transmitting…");
    try {
      await api("/api/rays", {
        method: "POST",
        body: JSON.stringify({
          frequency_id: Number(form.dataset.frequencyId),
          user_id: user.id,
          text: textarea.value,
        }),
      });
      window.location.reload();
    } catch (error) {
      setStatus(form, error.message, true);
    }
  });
}

function wirePrisms(user) {
  document.querySelectorAll(".prism-button").forEach((button) => {
    button.addEventListener("click", async () => {
      const rayId = Number(button.dataset.rayId);
      const prismed = button.getAttribute("aria-pressed") === "true";
      button.disabled = true;
      try {
        const result = prismed
          ? await api(`/api/prisms/${user.id}/${rayId}`, { method: "DELETE" })
          : await api("/api/prisms", {
              method: "POST",
              body: JSON.stringify({ user_id: user.id, ray_id: rayId }),
            });
        button.setAttribute("aria-pressed", String(result.prismed));
        button.classList.toggle("active", result.prismed);
        button.querySelector(".prism-icon").textContent = result.prismed ? "◆" : "◇";
        button.querySelector(".prism-count").textContent =
          `${result.prism_count} ${result.prism_count === 1 ? "prism" : "prisms"}`;
      } catch (error) {
        showToast(error.message, true);
      } finally {
        button.disabled = false;
      }
    });
  });
}

function wireUsernameForm(user) {
  const form = document.querySelector("#username-form");
  if (!form) return;
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const username = new FormData(form).get("username");
    setStatus(form, "Retuning identity…");
    try {
      const updated = await api(`/api/users/${user.id}`, {
        method: "PATCH",
        body: JSON.stringify({ username }),
      });
      localStorage.setItem(storageKey, JSON.stringify(updated));
      wireIdentity(updated);
      setStatus(form, "Identity updated.");
      showToast(`Now transmitting as ${updated.username}`);
    } catch (error) {
      setStatus(form, error.message, true);
    }
  });
}

function startBeamCanvas() {
  const canvas = document.querySelector("#beam-canvas");
  const passCounter = document.querySelector("#beam-pass-count");
  if (!canvas || !passCounter) return;
  const context = canvas.getContext("2d");
  if (!context) return;
  const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
  let pass = 0;
  let startedAt = performance.now();
  const duration = reduceMotion ? 8000 : 2600;

  function draw(now) {
    const elapsed = now - startedAt;
    if (elapsed >= duration) {
      pass += Math.floor(elapsed / duration);
      passCounter.textContent = String(pass);
      startedAt += Math.floor(elapsed / duration) * duration;
    }
    const progress = Math.min((now - startedAt) / duration, 1);
    const x = -36 + progress * (canvas.width + 72);
    context.clearRect(0, 0, canvas.width, canvas.height);
    context.fillStyle = "#10141d";
    context.fillRect(0, 0, canvas.width, canvas.height);
    context.strokeStyle = "#303849";
    context.lineWidth = 2;
    context.beginPath();
    context.moveTo(30, canvas.height / 2);
    context.lineTo(canvas.width - 30, canvas.height / 2);
    context.stroke();
    const glow = context.createRadialGradient(x, canvas.height / 2, 2, x, canvas.height / 2, 55);
    glow.addColorStop(0, "rgba(255, 70, 86, 1)");
    glow.addColorStop(0.25, "rgba(255, 38, 65, .7)");
    glow.addColorStop(1, "rgba(255, 38, 65, 0)");
    context.fillStyle = glow;
    context.fillRect(x - 60, canvas.height / 2 - 60, 120, 120);
    context.fillStyle = "#ff304f";
    context.fillRect(x - 13, canvas.height / 2 - 18, 26, 36);
    requestAnimationFrame(draw);
  }
  requestAnimationFrame(draw);
}

document.addEventListener("DOMContentLoaded", async () => {
  wireFrequencyForm();
  startBeamCanvas();
  try {
    const user = await loadIdentity();
    wireIdentity(user);
    wireRayForm(user);
    wirePrisms(user);
    wireUsernameForm(user);
  } catch (error) {
    showToast(`Could not initialize BeamRS: ${error.message}`, true);
    const username = document.querySelector("#current-username");
    if (username) username.textContent = "Offline";
  }
});
