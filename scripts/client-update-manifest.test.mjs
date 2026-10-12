import assert from "node:assert/strict";
import { mkdtempSync, rmSync, truncateSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { createUpdateManifest } from "./client-update-manifest.mjs";

function fixture(t, signedVersion = "1.5.0") {
	const assetsDir = mkdtempSync(join(tmpdir(), "mars-update-test-"));
	t.after(() => rmSync(assetsDir, { recursive: true }));
	for (const name of ["setup-1.5.0.exe", "mars-command-1.5.0.AppImage"]) {
		writeFileSync(join(assetsDir, name), "test artifact");
		const signature = Buffer.from(`untrusted comment: test fixture\nsignature\ntrusted comment: timestamp:1\tfile:${name}\tversion:${signedVersion}\ncomment signature\n`).toString("base64");
		writeFileSync(join(assetsDir, `${name}.sig`), signature);
	}
	return { version: "1.5.0", assetsDir, notes: "Test notes", pubDate: "2026-10-12T00:00:00Z" };
}

test("manifest publishes only versioned official Windows and Linux assets with inline signatures", (t) => {
	const options = fixture(t);
	const manifest = createUpdateManifest(options);
	assert.equal(manifest.version, "1.5.0");
	assert.equal(manifest.notes, "Test notes");
	assert.deepEqual(Object.keys(manifest.platforms), ["windows-x86_64", "linux-x86_64"]);
	for (const entry of Object.values(manifest.platforms)) {
		assert.ok(entry.url.startsWith("https://github.com/Mars-Command/Client-Launcher/releases/download/client-v1.5.0/"));
		assert.ok(Buffer.from(entry.signature, "base64").toString().includes("version:1.5.0"));
	}
});

test("missing signatures fail closed", (t) => {
	const options = fixture(t);
	rmSync(join(options.assetsDir, "setup-1.5.0.exe.sig"));
	assert.throws(() => createUpdateManifest(options), /ENOENT/);
});

test("wrong signed version is rejected", (t) => {
	assert.throws(() => createUpdateManifest(fixture(t, "1.4.0")), /not bound/);
});

test("unversioned legacy signatures are rejected", (t) => {
	const options = fixture(t);
	writeFileSync(join(options.assetsDir, "setup-1.5.0.exe.sig"), Buffer.from("trusted comment: timestamp:1\tfile:setup.exe\n").toString("base64"));
	assert.throws(() => createUpdateManifest(options), /not bound/);
});

test("invalid versions and empty artifacts are rejected", (t) => {
	const options = fixture(t);
	for (const version of ["", "../bad", "1.5.0-beta.1", "01.5.0", undefined]) {
		assert.throws(() => createUpdateManifest({ ...options, version }), /stable semantic/);
	}
	writeFileSync(join(options.assetsDir, "setup-1.5.0.exe"), "");
	assert.throws(() => createUpdateManifest(options), /artifact size/);
});

test("artifacts above the client's exact 256 MiB ceiling are rejected", (t) => {
	const options = fixture(t);
	const artifact = join(options.assetsDir, "setup-1.5.0.exe");
	truncateSync(artifact, 256 * 1024 * 1024);
	assert.doesNotThrow(() => createUpdateManifest(options));
	truncateSync(artifact, 256 * 1024 * 1024 + 1);
	assert.throws(() => createUpdateManifest(options), /artifact size/);
});
