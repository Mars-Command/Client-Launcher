import { Download, ShieldCheck } from "lucide-react";
import { useState } from "react";
import type { ClientUpdater } from "../hooks/useClientUpdater";
import { CLIENT_BUILD } from "../lib/mars";
import { Panel } from "./Panel";

export function ClientUpdatePanel({
	updater,
	blocked,
}: {
	updater: ClientUpdater;
	blocked: boolean;
}) {
	const [confirming, setConfirming] = useState(false);
	const busy = ["checking", "downloading", "installing"].includes(
		updater.phase,
	);
	const downloading =
		updater.progress?.event === "downloading"
			? updater.progress
			: null;
	const message =
		updater.phase === "checking"
			? "Checking the official release channel..."
			: updater.phase === "current"
				? "No newer client release is available."
				: updater.phase === "installing"
					? "Signature verified. Installing the client update..."
					: updater.phase === "downloading"
						? "Downloading the update. Installation waits for signature verification."
						: `Installed client: ${CLIENT_BUILD}`;

	return (
		<Panel title="Client updates" icon={<ShieldCheck size={14} />}>
			<div className="space-y-3 text-[11px] leading-4 text-slate-300">
				<p role="status">{message}</p>
				<p className="text-slate-400">
					Updates use HTTPS and must carry a valid
					signature from Mars Command. Checks
					never install anything automatically.
				</p>
				{updater.error ? (
					<p
						role="alert"
						className="text-red-200"
					>
						{updater.error} Check again to
						retry.
					</p>
				) : null}
				{downloading ? (
					<div>
						<progress
							className="w-full"
							aria-label="Client update download"
							value={
								downloading.total
									? downloading.downloaded
									: undefined
							}
							max={
								downloading.total ??
								undefined
							}
						/>
						<p>
							{(
								downloading.downloaded /
								1024 /
								1024
							).toFixed(1)}{" "}
							MiB downloaded
							{downloading.total
								? ` of ${(downloading.total / 1024 / 1024).toFixed(1)} MiB`
								: ""}
						</p>
					</div>
				) : null}
				{updater.update &&
				updater.phase === "available" ? (
					<div className="space-y-2 rounded-lg border border-cyan-200/20 bg-cyan-300/5 p-3">
						<p className="font-semibold text-cyan-100">
							Client{" "}
							{updater.update.version}{" "}
							is available
						</p>
						<p className="text-slate-400">
							Release notes are
							provided by the release
							channel; the downloaded
							installer is
							authenticated before it
							can run.
						</p>
						<pre className="max-h-40 overflow-y-auto whitespace-pre-wrap font-sans">
							{updater.update.notes ||
								"No release notes provided."}
						</pre>
						{confirming ? (
							<div
								role="group"
								aria-label="Confirm client update"
								className="space-y-2"
							>
								<p>
									Download
									and
									install{" "}
									{
										updater
											.update
											.version
									}
									? The
									client
									will
									close
									and
									restart.
									Finish
									any work
									first.
								</p>
								<div className="flex gap-2">
									<button
										type="button"
										disabled={
											blocked ||
											busy
										}
										onClick={() => {
											setConfirming(
												false,
											);
											void updater.install();
										}}
										className="rounded-lg bg-cyan-300/15 px-3 py-2 text-cyan-100 disabled:opacity-50"
									>
										Confirm
										download
										and
										install
									</button>
									<button
										type="button"
										onClick={() =>
											setConfirming(
												false,
											)
										}
										className="rounded-lg border border-white/10 px-3 py-2"
									>
										Cancel
									</button>
								</div>
							</div>
						) : (
							<button
								type="button"
								disabled={
									blocked ||
									busy
								}
								onClick={() =>
									setConfirming(
										true,
									)
								}
								className="flex items-center gap-2 rounded-lg bg-cyan-300/15 px-3 py-2 text-cyan-100 disabled:opacity-50"
							>
								<Download
									size={
										14
									}
								/>{" "}
								Review
								installation
							</button>
						)}
					</div>
				) : null}
				{blocked ? (
					<p className="text-amber-200">
						Finish the current game
						installation operation before
						updating the client.
					</p>
				) : null}
				<button
					type="button"
					disabled={busy}
					onClick={() => {
						setConfirming(false);
						void updater.check();
					}}
					className="rounded-lg border border-white/10 px-3 py-2 disabled:opacity-50"
				>
					Check for client updates
				</button>
			</div>
		</Panel>
	);
}
