"use client";

import Loading from "@/app/-loading";
import { CheckForUpdateMessage } from "@/components/CheckForUpdateMessage";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { LogEntry } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { DialogRoot, openSingleDialog } from "@/lib/dialog";
import { isFindKey, useDocumentEvent } from "@/lib/events";
import { tc } from "@/lib/i18n";
import { queryClient } from "@/lib/query-client";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { useTauriListen } from "@/lib/use-tauri-listen";
import { QueryClientProvider } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import type React from "react";
import { Suspense, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ToastContainer } from "react-toastify";

export function Providers({ children }: { children: React.ReactNode }) {
	const navigate = useNavigate();

	useTauriListen<LogEntry>("log", (event) => {
		const entry = event.payload as LogEntry;
		if (entry.level === "Error" && entry.gui_toast) {
			toastError(entry.message);
		}
	});

	const moveToRepositories = useCallback(() => {
		if (location.pathname !== "/packages/repositories") {
			navigate({ to: "/packages/repositories" });
		}
	}, [navigate]);

	useTauriListen<null>("deep-link-add-repository", (_) => {
		moveToRepositories();
	});

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

	useTauriListen<number>("templates-imported", async ({ payload: count }) => {
		await queryClient.invalidateQueries({
			queryKey: ["environmentProjectCreationInformation"],
		});
		toastSuccess(tc("templates:toast:imported n templates", { count }));
	});

	useEffect(() => {
		(async () => {
			const count = await commands.deepLinkImportedClearNonToastedCount();
			if (count !== 0) {
				toastSuccess(tc("templates:toast:imported n templates", { count }));
			}
		})();
	}, []);

	const { i18n } = useTranslation();

	useEffect(() => {
		let cancel = false;
		(async () => {
			try {
				if (import.meta.env.DEV) return;
				const checkVersion = await commands.utilCheckForUpdate();
				if (cancel) return;
				if (checkVersion) {
					await openSingleDialog(CheckForUpdateMessage, {
						response: checkVersion,
					});
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
					<div lang={i18n.language} className="contents">
						<Suspense fallback={<Loading />}>{children}</Suspense>
					</div>
					<DialogRoot />
				</TooltipProvider>
			</QueryClientProvider>
		</>
	);
}
