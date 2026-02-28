/**
 * Custom Footer Extension
 *
 * Reproduces the default pi footer faithfully and adds a dedicated
 * session-name line at the top, bold and accent-colored, only when set.
 *
 * Based on the default footer source at:
 *   .../dist/modes/interactive/components/footer.js
 */

import type { AssistantMessage } from "@mariozechner/pi-ai";
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { truncateToWidth, visibleWidth } from "@mariozechner/pi-tui";

function sanitizeStatusText(text: string): string {
	return text.replace(/[\r\n\t]/g, " ").replace(/ +/g, " ").trim();
}

function formatTokens(count: number): string {
	if (count < 1000) return count.toString();
	if (count < 10_000) return `${(count / 1000).toFixed(1)}k`;
	if (count < 1_000_000) return `${Math.round(count / 1000)}k`;
	if (count < 10_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
	return `${Math.round(count / 1_000_000)}M`;
}

export default function (pi: ExtensionAPI) {
	pi.on("session_start", async (_event, ctx) => {
		ctx.ui.setFooter((tui, theme, footerData) => {
			const unsub = footerData.onBranchChange(() => tui.requestRender());

			return {
				dispose: unsub,
				invalidate() {},
				render(width: number): string[] {
					const lines: string[] = [];

					// ── Location bar ─────────────────────────────────────
					let pwd = process.cwd();
					const home = process.env.HOME || process.env.USERPROFILE;
					if (home && pwd.startsWith(home)) {
						pwd = `~${pwd.slice(home.length)}`;
					}

					const branch = footerData.getGitBranch();
					if (branch) {
						pwd = `${pwd} (${branch})`;
					}

					// Session name: bold + accent, no quotes
					const rawSessionName = pi.getSessionName();
					const sessionName = rawSessionName?.replace(/^"|"$/g, "");
					const sessionNameSuffix = sessionName
						? " • " + theme.fg("accent", theme.bold(sessionName))
						: "";

					// Smart path truncation (start...end) like the default
					if (pwd.length > width) {
						const half = Math.floor(width / 2) - 2;
						if (half > 1) {
							pwd = `${pwd.slice(0, half)}...${pwd.slice(-(half - 1))}`;
						} else {
							pwd = pwd.slice(0, Math.max(1, width));
						}
					}

					lines.push(truncateToWidth(theme.fg("dim", pwd) + sessionNameSuffix, width));

					// ── Stats bar ────────────────────────────────────────
					// Cumulative usage from ALL entries (not just branch)
					let totalInput = 0;
					let totalOutput = 0;
					let totalCacheRead = 0;
					let totalCacheWrite = 0;
					let totalCost = 0;

					for (const entry of ctx.sessionManager.getEntries()) {
						if (entry.type === "message" && entry.message.role === "assistant") {
							const m = entry.message as AssistantMessage;
							totalInput += m.usage.input;
							totalOutput += m.usage.output;
							totalCacheRead += m.usage.cacheRead;
							totalCacheWrite += m.usage.cacheWrite;
							totalCost += m.usage.cost.total;
						}
					}

					const statsParts: string[] = [];
					if (totalInput) statsParts.push(`↑${formatTokens(totalInput)}`);
					if (totalOutput) statsParts.push(`↓${formatTokens(totalOutput)}`);
					if (totalCacheRead) statsParts.push(`R${formatTokens(totalCacheRead)}`);
					if (totalCacheWrite) statsParts.push(`W${formatTokens(totalCacheWrite)}`);

					// Cost (with subscription indicator if using OAuth)
					const usingSubscription = ctx.model
						? ctx.modelRegistry.isUsingOAuth(ctx.model)
						: false;
					if (totalCost || usingSubscription) {
						const costStr = `$${totalCost.toFixed(3)}${usingSubscription ? " (sub)" : ""}`;
						statsParts.push(costStr);
					}

					// Context percentage — colorized by usage level
					const contextUsage = ctx.getContextUsage();
					const contextWindow = contextUsage?.contextWindow ?? ctx.model?.contextWindow ?? 0;
					const contextPercentValue = contextUsage?.percent ?? 0;
					const contextPercent =
						contextUsage?.percent !== null ? contextPercentValue.toFixed(1) : "?";

					const contextPercentDisplay =
						contextPercent === "?"
							? `?/${formatTokens(contextWindow)}`
							: `${contextPercent}%/${formatTokens(contextWindow)}`;

					let contextPercentStr: string;
					if (contextPercentValue > 90) {
						contextPercentStr = theme.fg("error", contextPercentDisplay);
					} else if (contextPercentValue > 70) {
						contextPercentStr = theme.fg("warning", contextPercentDisplay);
					} else {
						contextPercentStr = contextPercentDisplay;
					}
					statsParts.push(contextPercentStr);

					let statsLeft = statsParts.join(" ");
					let statsLeftWidth = visibleWidth(statsLeft);

					// Truncate left side if too wide
					if (statsLeftWidth > width) {
						const plain = statsLeft.replace(/\x1b\[[0-9;]*m/g, "");
						statsLeft = `${plain.substring(0, width - 3)}...`;
						statsLeftWidth = visibleWidth(statsLeft);
					}

					// Right side: [provider] model • thinking-level
					const modelName = ctx.model?.id || "no-model";
					const thinkingLevel = pi.getThinkingLevel();

					let rightSideBase = modelName;
					if (ctx.model?.reasoning && thinkingLevel !== "off") {
						rightSideBase = `${modelName} • ${thinkingLevel}`;
					} else if (ctx.model?.reasoning) {
						rightSideBase = `${modelName} • thinking off`;
					}

					// Add provider prefix when multiple providers available
					let rightSide = rightSideBase;
					const minPadding = 2;
					if (footerData.getAvailableProviderCount() > 1 && ctx.model) {
						const withProvider = `(${ctx.model.provider}) ${rightSideBase}`;
						if (statsLeftWidth + minPadding + visibleWidth(withProvider) <= width) {
							rightSide = withProvider;
						}
					}

					const rightSideWidth = visibleWidth(rightSide);
					const totalNeeded = statsLeftWidth + minPadding + rightSideWidth;

					let statsLine: string;
					if (totalNeeded <= width) {
						const padding = " ".repeat(width - statsLeftWidth - rightSideWidth);
						statsLine = statsLeft + padding + rightSide;
					} else {
						const availableForRight = width - statsLeftWidth - minPadding;
						if (availableForRight > 3) {
							const plainRight = rightSide.replace(/\x1b\[[0-9;]*m/g, "");
							const truncated = plainRight.substring(0, availableForRight);
							const padding = " ".repeat(width - statsLeftWidth - truncated.length);
							statsLine = statsLeft + padding + truncated;
						} else {
							statsLine = statsLeft;
						}
					}

					// Apply dim to left and right separately (context % may have its own colors)
					const dimLeft = theme.fg("dim", statsLeft);
					const remainder = statsLine.slice(statsLeft.length);
					const dimRemainder = theme.fg("dim", remainder);
					lines.push(dimLeft + dimRemainder);

					// ── Extension statuses (own line at bottom) ──────────
					const extensionStatuses = footerData.getExtensionStatuses();
					if (extensionStatuses.size > 0) {
						const sortedStatuses = Array.from(extensionStatuses.entries())
							.sort(([a], [b]) => a.localeCompare(b))
							.map(([, text]) => sanitizeStatusText(text));
						const statusLine = sortedStatuses.join(" ");
						lines.push(truncateToWidth(statusLine, width, theme.fg("dim", "...")));
					}

					return lines;
				},
			};
		});
	});
}
