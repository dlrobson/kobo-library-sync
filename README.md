# Kobo Library Sync

WIP. The goal is to create a tool that will sync your Kobo eReader library with
a folder on your computer. This will have a docker image with a rest interface
that the Kobo eReader can connect to.

## CI/CD

CI/CD performs automatic dependency updates through renovatebot, and runs tests
on each pull request. A `RENOVATE_TOKEN` secret is required for the dependency
updates to work.
[See here for more information](https://github.com/renovatebot/github-action/blob/adad17015c735c8b1f417ddf1f7f19750a140881/README.md#token).
