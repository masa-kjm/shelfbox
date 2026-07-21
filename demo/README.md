# shelfbox demo sandbox

This directory provides two disposable Linux environments:

- `verify` is a small Git + shelfbox sandbox for trying CLI workflows.
- `record` adds VHS, Chromium, ffmpeg, and fonts solely to regenerate the GIF
  embedded in the repository README.

The two images are deliberately separate, so a manual verification does not
download the GIF recorder dependencies.

## Prerequisites

- Docker Engine or Docker Desktop, with the Compose plugin

## Regenerate the README GIF

Run this command from the repository root. The two environment variables make files created through the bind mount belong to the invoking user on Linux.

```sh
DEMO_UID="$(id -u)" DEMO_GID="$(id -g)" \
  docker compose -f demo/compose.yml run --rm record
```

The first run builds shelfbox and the VHS recorder image. It recreates the
ignored `demo/workspace/` directory and writes the tracked output to
`demo/output/README.gif`.

Build again after changing Rust sources:

```sh
docker compose -f demo/compose.yml build record
```

## Try shelfbox interactively

Start a shell in the lightweight `verify` environment:

```sh
DEMO_UID="$(id -u)" DEMO_GID="$(id -g)" \
  docker compose -f demo/compose.yml run --rm verify
```

Then create a clean sample repository and store:

```sh
./setup.sh --reset
cd workspace/repo
shelfbox --store ../store item add .env
shelfbox --store ../store item status
```

After changing Rust sources, rebuild only this lightweight image before
starting the shell:

```sh
docker compose -f demo/compose.yml build verify
```

`setup.sh --reset` deletes only `demo/workspace/`, which is generated and ignored by Git. It never reads or writes a real user store or repository.
