Use ./bin/yx to plan the work below in the current repository, starting from
the empty map. Use that executable for every command; it is the version being
evaluated. Read its help and any command help you need. Only plan the work;
don't implement the service or mark unfinished work done.

We're trying to get our catalog service launched. The work touches three
codebases: the common SDK, our existing importer, and the catalog HTTP service.
The people maintaining each codebase will pick up their own part of the work.

The HTTP service needs a new search endpoint. The importer needs adapting
to use the common SDK. In the SDK, the reader and refresh paths are still
stubs, so the package hasn't been validated yet. The importer maintainer
won't migrate onto an unvalidated SDK. The release owner will wire the adapted
importer and the search endpoint together once both work, then deploy that
combination for launch.

There's also the shared contract to agree and publish. It's the agreement
that the search endpoint and both SDK paths need before anyone can implement
those consumers. Reader and refresh talk to the vendor API; the vendor hasn't
sent credentials, so neither implementation can begin yet.

Turn this into a useful yak map so another developer can pick up ready work
and understand how it gets us to launch. Choose sensible names for the yaks.
