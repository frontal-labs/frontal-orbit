# orbit-repo

Repo lifecycle primitives for hosted execution.

This crate is the boundary between connector/control-plane code and source-tree
preparation. It owns local checkout preparation concerns like clone, fetch,
base-ref resolution, and branch creation/reset. It is intentionally GitHub-agnostic.
