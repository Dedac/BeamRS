---
status: active
phase: phasing
---

# Phasing

1. Inventory: inspect approved artifacts, direct/transitive dependencies,
   APIs, and build metadata.
2. Implement: update direct versions and lockfile, then resolve only
   compiler-identified compatibility issues.
3. Integrate: review the complete diff for unrelated changes and update
   directly related documentation.
4. Test: run the required focused and repository validation commands.
5. Replan: record any migration issues and final outcomes before commit.
