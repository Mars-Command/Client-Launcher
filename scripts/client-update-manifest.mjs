import { readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export function createUpdateManifest({ version, assetsDir, notes, pubDate }) {
	if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
		throw new Error("Client release version must be a stable semantic version");
	}
	if (!Number.isFinite(Date.parse(pubDate))) throw new Error("Release date is invalid");
	const platforms = {};
	for (const [target, name] of [
		["windows-x86_64", `setup-${version}.exe`],
		["linux-x86_64", `mars-command-${version}.AppImage`],
	]) {
		const artifact = join(assetsDir, name);
		const size = statSync(artifact).size;
		if (size === 0 || size > 256 * 1024 * 1024) throw new Error(`Invalid update artifact size: ${name}`);
		const signature = readFileSync(`${artifact}.sig`, "utf8").trim();
		if (!/^[A-Za-z0-9+/]+={0,2}$/.test(signature)) throw new Error(`Malformed signature for ${name}`);
		const decoded = Buffer.from(signature, "base64").toString("utf8");
		const comment = decoded.split(/\r?\n/).find((line) => line.startsWith("trusted comment: "));
		const signedVersion = comment?.slice("trusted comment: ".length).split("\t").find((part) => part.startsWith("version:"))?.slice("version:".length);
		if (signedVersion !== version) throw new Error(`Signature is not bound to release ${version}: ${name}`);
		platforms[target] = {
			url: `https://github.com/Mars-Command/Client-Launcher/releases/download/client-v${version}/${name}`,
			signature,
		};
	}
	return { version, notes, pub_date: pubDate, platforms };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	const version = process.env.GITHUB_REF_NAME?.replace(/^client-v/, "");
	const assetsDir = process.argv[2] ?? "release-assets";
	const manifest = createUpdateManifest({
		version,
		assetsDir,
		notes: `Mars Command ${version}. Release details: https://github.com/Mars-Command/Client-Launcher/releases/tag/client-v${version}`,
		pubDate: new Date().toISOString(),
	});
	writeFileSync(join(assetsDir, "latest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
}
