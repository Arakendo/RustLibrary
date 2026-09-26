# Change Management

This optional category contains durable instructions and policy for changes
that affect users, operators, stored data, compatibility, or supported
workflows.

- [`migrations/`](migrations/) describes how to move between versions or
  architectures.
- [`deprecations/`](deprecations/) announces supported retirement paths.

Implementation sequencing belongs in [`plans/`](../plans/). Completed migration
instructions may remain current for supported upgrade paths; archive them only
after the source version is no longer supported.
