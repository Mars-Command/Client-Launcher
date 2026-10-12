import { Channel, invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

type UpdateInfo = { version: string; notes: string | null };
type Progress =
	| { event: "downloading"; downloaded: number; total: number | null }
	| { event: "installing" };
type Phase = "idle" | "checking" | "available" | "current" | "downloading" | "installing" | "error";

export function useClientUpdater() {
	const [phase, setPhase] = useState<Phase>("idle");
	const [update, setUpdate] = useState<UpdateInfo | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [progress, setProgress] = useState<Progress | null>(null);
	const busy = useRef(false);
	const mounted = useRef(false);
	const started = useRef(false);

	const check = useCallback(async () => {
		if (busy.current) return;
		busy.current = true;
		setPhase("checking");
		setError(null);
		setUpdate(null);
		setProgress(null);
		try {
			const available = await invoke<UpdateInfo | null>("check_client_update");
			if (mounted.current) {
				setUpdate(available);
				setPhase(available ? "available" : "current");
			}
		} catch (reason) {
			if (mounted.current) {
				setError(String(reason));
				setPhase("error");
			}
		} finally {
			busy.current = false;
		}
	}, []);

	const install = useCallback(async () => {
		if (busy.current || !update || phase !== "available") return;
		busy.current = true;
		setPhase("downloading");
		setError(null);
		const channel = new Channel<Progress>();
		channel.onmessage = (message) => {
			if (!mounted.current) return;
			setProgress(message);
			setPhase(message.event === "installing" ? "installing" : "downloading");
		};
		try {
			await invoke("install_client_update", {
				expectedVersion: update.version,
				confirmed: true,
				progress: channel,
			});
		} catch (reason) {
			if (mounted.current) {
				setUpdate(null);
				setError(String(reason));
				setPhase("error");
			}
		} finally {
			busy.current = false;
		}
	}, [update, phase]);

	useEffect(() => {
		mounted.current = true;
		if (!started.current) {
			started.current = true;
			void check();
		}
		return () => { mounted.current = false; };
	}, [check]);

	return { phase, update, error, progress, check, install };
}

export type ClientUpdater = ReturnType<typeof useClientUpdater>;
