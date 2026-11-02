Use ./bin/yx to create a yak map for this work in the current repository.
Use that exact executable for every command; it is the version being evaluated.
Start by reading ./bin/yx --help and any command help you need.

- The outcome is to launch a catalog service.
- Deploying the catalog service can happen after its components are wired together.
- Wiring requires both adapting a legacy importer and building a search endpoint.
- Adapting the legacy importer requires validating a common library.
- Validating the common library requires implementing its reader and refresh paths.
- A shared contract must be published before the reader, refresh path, or search endpoint can be implemented.
- Both the reader and refresh implementation tasks need vendor credentials, which have not arrived.

Use these exact yak names so the result can be checked automatically:
Launch catalog service, Deploy catalog service, Wire components,
Adapt legacy importer, Build search endpoint, Validate common library,
Implement reader, Implement refresh, Publish shared contract.

Create the map using yx commands. Leave the work unfinished; this task is planning only.
Do not implement the service. Finish with a brief explanation of the map you created.
