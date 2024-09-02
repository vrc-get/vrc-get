"use client";

import "@/lib/polyfill";
import { CheckForUpdateMessage } from "@/components/CheckForUpdateMessage";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { CheckForUpdateResponse, LogEntry } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { isFindKey, useDocumentEvent } from "@/lib/events";
import { toastError, toastThrownError } from "@/lib/toast";
import { useTauriListen } from "@/lib/use-tauri-listen";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import type React from "react";
import { Suspense, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ToastContainer } from "react-toastify";

const queryClient = new QueryClient();

export function Providers({ children }: { children: React.ReactNode }) {
	const router = useRouter();

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
			router.push("/packages/repositories");
		}
	}, [router]);

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
				const isDev = process.env.NODE_ENV === "development";
				if (isDev) return;
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
						<Suspense fallback={"Loading..."}>{children}</Suspense>
					</div>
				</TooltipProvider>
			</QueryClientProvider>
		</>
	);
}
