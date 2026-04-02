# TODO

This file tracks follow-up tasks and cleanups that came up during planning but are not currently part of an active exec plan.

- Make per-file ignore matching inherit directory patterns for descendants by using `ignore::gitignore::Gitignore::matched_path_or_any_parents`, so patterns like `docs/references` behave like users expect for files under that directory.
