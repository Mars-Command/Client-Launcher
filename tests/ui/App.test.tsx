import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../../src/App";

const mocks = vi.hoisted(() => ({
	account: {
		user: null as { id: string; username: string; avatarUrl: string; roles: string[] } | null,
		pending: false,
		message: "",
		login: vi.fn(async () => {}),
		logout: vi.fn(async () => {}),
		donate: vi.fn(async () => {}),
		refresh: vi.fn(async () => null),
		sessionEpoch: 0,
	},
}));

vi.mock("../../src/hooks/useCommunityAccount", () => ({ useCommunityAccount: () => mocks.account }));
vi.mock("../../src/hooks/useMissionControlConfig", () => ({
	useMissionControlConfig: () => ({ target: { host: "test.example", port: 25565 }, updateTarget: vi.fn(), storageError: null }),
}));
vi.mock("../../src/hooks/useMarsServerStatus", () => ({
	useMarsServerStatus: () => ({ status: null, loading: false, refreshing: false, refresh: vi.fn(), lastSuccessfulCheck: null }),
}));
vi.mock("../../src/hooks/usePackIntegrity", () => ({ usePackIntegrity: () => ({ manifest: null, report: null }) }));
vi.mock("../../src/hooks/useClientUpdater", () => ({
	useClientUpdater: () => ({ phase: "available", update: { version: "1.4.2" } }),
}));
vi.mock("@tauri-apps/api/window", () => ({
	getCurrentWindow: () => ({ startDragging: vi.fn(), toggleMaximize: vi.fn(), close: vi.fn(), minimize: vi.fn() }),
}));
vi.mock("../../src/components/DeploymentPanel", () => ({ DeploymentPanel: () => <h1>Deployment Status</h1> }));
vi.mock("../../src/components/ServerStatusPanel", () => ({ ServerStatusPanel: () => <h2>Server Status</h2> }));
vi.mock("../../src/components/MissionControlPanel", () => ({
	MissionControlPanel: () => <h2>Mission Target</h2>, CrewChannelPanel: () => <h2>Crew Channel</h2>,
}));
vi.mock("../../src/components/CommunityPage", () => ({ CommunityPage: () => <h1>Community profiles</h1> }));
vi.mock("../../src/components/SettingsPage", () => ({
	SettingsPage: ({ onDonate }: { onDonate: () => void }) => <section><h1>Client Settings</h1><button onClick={onDonate}>Fund This Project</button></section>,
}));

let container: HTMLDivElement;
let root: Root;

beforeEach(async () => {
	(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
	mocks.account.user = null;
	mocks.account.pending = false;
	mocks.account.message = "";
	vi.clearAllMocks();
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
	await act(async () => root.render(<App />));
});

afterEach(async () => {
	await act(async () => root.unmount());
	container.remove();
});

function button(name: string) {
	const result = [...container.querySelectorAll("button")].find(element =>
		element.getAttribute("aria-label") === name || element.textContent === name);
	if (!result) throw new Error(`Missing button: ${name}`);
	return result;
}

async function click(name: string) {
	await act(async () => button(name).click());
}

describe("dedicated profile navigation", () => {
	it("replaces the dashboard rather than appending an account card", async () => {
		await click("Open website account login");
		expect(container.querySelector("#account-page-title")?.textContent).toBe("Your Profile");
		expect(container.textContent).not.toContain("Deployment Status");
		expect(container.textContent).not.toContain("Server Status");
		expect(container.textContent).not.toContain("Client Settings");
		expect(button("Close profile").getAttribute("aria-pressed")).toBe("true");
		expect(button("Mission control").getAttribute("aria-pressed")).toBe("false");
		expect(mocks.account.login).toHaveBeenCalledOnce();
	});

	it("returns to the previous community page when the profile is closed", async () => {
		await click("Community");
		await click("Open website account login");
		expect(container.textContent).not.toContain("Community profiles");
		await click("Close profile");
		expect(container.textContent).toContain("Community profiles");
		expect(container.querySelector("#account-page-title")).toBeNull();
	});

	it("opens a profile from settings and restores settings on close", async () => {
		await click("Open settings");
		await click("Open website account login");
		expect(container.textContent).not.toContain("Client Settings");
		await click("Close profile");
		expect(container.textContent).toContain("Client Settings");
		expect(container.querySelector("#account-page-title")).toBeNull();
	});

	it("navigates from profile to update settings without retaining the card", async () => {
		await click("Open website account login");
		await click("Update settings");
		expect(container.textContent).toContain("Client Settings");
		expect(container.querySelector("#account-page-title")).toBeNull();
		expect(container.textContent).not.toContain("Website account");
	});

	it("shows donation and login failures on the dedicated profile page", async () => {
		await click("Open settings");
		await click("Fund This Project");
		expect(mocks.account.donate).toHaveBeenCalledOnce();
		expect(container.querySelector("#account-page-title")).not.toBeNull();
		mocks.account.message = "Could not start website login";
		await act(async () => root.render(<App />));
		expect(container.querySelector('[role="status"][aria-live="polite"]')?.textContent)
			.toBe("Could not start website login");
		expect(container.textContent).not.toContain("Client Settings");
	});

	it("preserves an existing account and does not restart pending login on navigation", async () => {
		mocks.account.pending = true;
		await act(async () => root.render(<App />));
		await click("Open website account login");
		expect(mocks.account.login).not.toHaveBeenCalled();
		await click("Close profile");
		mocks.account.pending = false;
		mocks.account.user = { id: "github:42", username: "astronaut", avatarUrl: "", roles: [] };
		await act(async () => root.render(<App />));
		await click("Account: astronaut");
		expect(container.textContent).toContain("GitHub: astronaut");
		expect(mocks.account.login).not.toHaveBeenCalled();
	});
});
