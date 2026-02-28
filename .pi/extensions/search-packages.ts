/**
 * Search Pi Packages - Browse and install pi packages from npm
 *
 * Provides a /search-packages command that queries the npm registry
 * for packages with the "pi-package" keyword and displays results
 * in a scrollable, searchable list.
 *
 * Usage: /search-packages [query]
 */

import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { BorderedLoader, DynamicBorder } from "@mariozechner/pi-coding-agent";
import { Container, type SelectItem, SelectList, Text } from "@mariozechner/pi-tui";

interface NpmPackage {
	name: string;
	version: string;
	description: string;
	author: string;
	keywords: string[];
	date: string;
	weeklyDownloads: number;
	npmUrl: string;
}

async function searchNpm(query: string, signal?: AbortSignal): Promise<NpmPackage[]> {
	const searchText = query ? `keywords:pi-package ${query}` : "keywords:pi-package";
	const url = `https://registry.npmjs.org/-/v1/search?text=${encodeURIComponent(searchText)}&size=100`;

	const response = await fetch(url, { signal });
	if (!response.ok) {
		throw new Error(`npm search failed: ${response.status} ${response.statusText}`);
	}

	const data = (await response.json()) as {
		objects: Array<{
			package: {
				name: string;
				version: string;
				description?: string;
				keywords?: string[];
				publisher?: { username?: string };
				date?: string;
				links?: { npm?: string };
			};
			downloads?: { weekly?: number };
		}>;
	};

	return data.objects.map((obj) => ({
		name: obj.package.name,
		version: obj.package.version,
		description: obj.package.description || "",
		author: obj.package.publisher?.username || "unknown",
		keywords: (obj.package.keywords || []).filter((k) => k !== "pi-package"),
		date: obj.package.date ? new Date(obj.package.date).toLocaleDateString() : "",
		weeklyDownloads: obj.downloads?.weekly || 0,
		npmUrl: obj.package.links?.npm || `https://www.npmjs.com/package/${obj.package.name}`,
	}));
}

function formatDownloads(n: number): string {
	if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
	return String(n);
}

export default function (pi: ExtensionAPI) {
	pi.registerCommand("search-packages", {
		description: "Search npm for pi packages",
		handler: async (args, ctx) => {
			if (!ctx.hasUI) {
				ctx.ui.notify("search-packages requires interactive mode", "error");
				return;
			}

			// Phase 1: Search with a loader
			const packages = await ctx.ui.custom<NpmPackage[] | null>((tui, theme, _kb, done) => {
				const searchQuery = args?.trim() || "";
				const label = searchQuery
					? `Searching npm for pi packages matching "${searchQuery}"...`
					: "Searching npm for pi packages...";
				const loader = new BorderedLoader(tui, theme, label);
				loader.onAbort = () => done(null);

				searchNpm(searchQuery, loader.signal)
					.then((results) => done(results))
					.catch((err) => {
						if (err.name !== "AbortError") {
							ctx.ui.notify(`Search failed: ${err.message}`, "error");
						}
						done(null);
					});

				return loader;
			});

			if (!packages || packages.length === 0) {
				if (packages) ctx.ui.notify("No packages found", "warning");
				return;
			}

			// Phase 2: Display results in a SelectList
			const selected = await ctx.ui.custom<NpmPackage | null>((tui, theme, _kb, done) => {
				const container = new Container();

				container.addChild(new DynamicBorder((s: string) => theme.fg("accent", s)));

				const title = `Found ${packages.length} pi package${packages.length === 1 ? "" : "s"}`;
				container.addChild(new Text(theme.fg("accent", theme.bold(title)), 1, 0));
				container.addChild(new Text("", 0, 0));

				const items: SelectItem[] = packages.map((pkg) => ({
					value: pkg.name,
					label: `${pkg.name} ${theme.fg("dim", `v${pkg.version}`)}`,
					description: [
						pkg.description,
						theme.fg("dim", `↓${formatDownloads(pkg.weeklyDownloads)}/wk`),
						theme.fg("dim", `by ${pkg.author}`),
						pkg.date ? theme.fg("dim", pkg.date) : "",
					]
						.filter(Boolean)
						.join("  "),
				}));

				const selectList = new SelectList(items, Math.min(items.length, 15), {
					selectedPrefix: (t: string) => theme.fg("accent", t),
					selectedText: (t: string) => theme.fg("accent", t),
					description: (t: string) => t, // already themed
					scrollInfo: (t: string) => theme.fg("dim", t),
					noMatch: (t: string) => theme.fg("warning", t),
				});

				selectList.onSelect = (item) => {
					const pkg = packages.find((p) => p.name === item.value) || null;
					done(pkg);
				};
				selectList.onCancel = () => done(null);

				container.addChild(selectList);
				container.addChild(
					new Text(theme.fg("dim", "↑↓ navigate • type to filter • enter select • esc cancel"), 1, 0),
				);
				container.addChild(new DynamicBorder((s: string) => theme.fg("accent", s)));

				return {
					render: (w: number) => container.render(w),
					invalidate: () => container.invalidate(),
					handleInput: (data: string) => {
						selectList.handleInput(data);
						tui.requestRender();
					},
				};
			});

			if (!selected) return;

			// Phase 3: Show package details and offer to install
			const action = await ctx.ui.select(`${selected.name} v${selected.version}`, [
				"Install globally",
				"Install for this project",
				"Copy install command",
				"Cancel",
			]);

			if (!action || action === "Cancel") return;

			const npmSource = `npm:${selected.name}`;

			if (action === "Copy install command") {
				ctx.ui.setEditorText(`pi install ${npmSource}`);
				ctx.ui.notify("Install command placed in editor", "info");
				return;
			}

			const localFlag = action === "Install for this project" ? " -l" : "";
			const installCmd = `pi install ${npmSource}${localFlag}`;

			ctx.ui.notify(`Running: ${installCmd}`, "info");
			const result = await pi.exec("pi", ["install", npmSource, ...(localFlag ? ["-l"] : [])]);

			if (result.code === 0) {
				ctx.ui.notify(`✓ Installed ${selected.name}`, "info");
			} else {
				ctx.ui.notify(`Install failed (exit ${result.code}): ${result.stderr || result.stdout}`, "error");
			}
		},
	});
}
