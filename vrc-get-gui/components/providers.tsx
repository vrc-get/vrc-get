"use client";

import Loading from "@/app/-loading";
import { CheckForUpdateMessage } from "@/components/CheckForUpdateMessage";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { CheckForUpdateResponse, LogEntry } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { DialogRoot } from "@/lib/dialog";
import { isFindKey, useDocumentEvent } from "@/lib/events";
import { queryClient } from "@/lib/query-client";
import { toastError, toastThrownError } from "@/lib/toast";
import { useTauriListen } from "@/lib/use-tauri-listen";
import { QueryClientProvider } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import type React from "react";
import { Suspense, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ToastContainer } from "react-toastify";

export function Providers({ children }: { children: React.ReactNode }) {
	const navigate = useNavigate();

	useTauriListen<LogEntry>(
		"log",
		useCallback((event) => {
			const entry = event.payload as LogEntry;
			if (entry.level === "Error" && entry.gui_toast) {
				toastError(entry.message);
			}
		}, []),
	);

	const moveToRepositories = useCallback(() => {
		if (location.pathname !== "/packages/repositories") {
			navigate({ to: "/packages/repositories" });
		}
	}, [navigate]);

	useTauriListen<null>(
		"deep-link-add-repository",
		useCallback(
			(_) => {
				moveToRepositories();
			},
			[moveToRepositories],
		),
	);

	useEffect(() => {
		let cancel = false;
		commands.deepLinkHasAddRepository().then((has) => {
			if (cancel) return;
			if (has) {
				moveToRepositories();
			}
		});
		return () => {
			cancel = true;
		};
	}, [moveToRepositories]);

	const { i18n } = useTranslation();

	const [updateState, setUpdateState] = useState<CheckForUpdateResponse | null>(
		null,
	);

	useEffect(() => {
		let cancel = false;
		(async () => {
			try {
				if (import.meta.env.DEV) return;
				const checkVersion = await commands.utilCheckForUpdate();
				if (cancel) return;
				if (checkVersion) {
					setUpdateState(checkVersion);
				}
			} catch (e) {
				toastThrownError(e);
				console.error(e);
			}
		})();
		return () => {
			cancel = true;
		};
	}, []);

	useDocumentEvent(
		"keydown",
		(e) => {
			if (isFindKey(e)) {
				e.preventDefault();
			}
		},
		[],
	);

	return (
		<>
			<ToastContainer
				position="bottom-right"
				autoClose={3000}
				hideProgressBar={false}
				newestOnTop={false}
				closeOnClick
				rtl={false}
				pauseOnFocusLoss
				draggable
				pauseOnHover
				theme="light"
				className={"whitespace-normal"}
			/>
			<QueryClientProvider client={queryClient}>
				<TooltipProvider>
					{updateState && (
						<CheckForUpdateMessage
							response={updateState}
							close={() => setUpdateState(null)}
						/>
					)}
					<div lang={i18n.language} className="contents">
						<Suspense fallback={<Loading />}>{children}</Suspense>
					</div>
					<DialogRoot />
				</TooltipProvider>
			</QueryClientProvider>
		</>
	);
}
