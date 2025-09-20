# Dev Container

This is a development container for the project. When the devcontainer
initializes for the first time, a `compose.override.yaml` file will be created
in the .devcontainer directory. This file will be used to override the
`docker-compose.yml` file in the root of the project with user specific
settings such as mounting the user's history file.