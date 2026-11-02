#!/usr/bin/env bash
set -euo pipefail

yx add "Launch catalog service"
yx add "Publish shared contract" --under "Launch catalog service"
yx add "Deploy catalog service" --under "Launch catalog service"
yx add "Wire components" --under "Deploy catalog service"
yx add "Adapt legacy importer" --under "Wire components"
yx add "Validate common library" --under "Adapt legacy importer"
yx add "Implement reader" --under "Validate common library"
yx add "Implement refresh" --under "Validate common library"
yx add "Build search endpoint" --under "Wire components"
yx blocker add "Implement reader" --by "Publish shared contract"
yx blocker add "Implement refresh" --by "Publish shared contract"
yx blocker add "Build search endpoint" --by "Publish shared contract"
yx blocker add "Implement reader" --reason "Waiting for vendor credentials"
yx blocker add "Implement refresh" --reason "Waiting for vendor credentials"
