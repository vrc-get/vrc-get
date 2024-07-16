"use client";

import { CheckForUpdateMessage } from "@/components/CheckForUpdateMessage";
import { TooltipProvider } from "@/components/ui/tooltip";
import {
	type CheckForUpdateResponse,
	type LogEntry,
	deepLinkHasAddRepository,
	utilCheckForUpdate,
} from "@/lib/bindings";
import i18next from "@/lib/i18n";
import { toastError, toastThrownError } from "@/lib/toast";
import { useTauriListen } from "@/lib/use-tauri-listen";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { I18nextProvider } from "react-i18next";
import { ToastContainer } from "react-toastify";

const queryClient = new QueryClient();

export function Providers({ children }: { children: React.ReactNode }) {
	const router = useRouter();

	useTauriListen<LogEntry>(
		"log",
		useCallback((event) => {
			const entry = event.payload as LogEntry;
			if (entry.level === "Error") {
				toastError(entry.message);
			}
		}, []),
	);

	const moveToRepositories = useCallback(() => {
		if (location.pathname !== "/repositories") {
			router.push("/repositories");
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
		deepLinkHasAddRepository().then((has) => {
			if (cancel) return;
			if (has) {
				moveToRepositories();
			}
		});
		return () => {
			cancel = true;
		};
	}, [moveToRepositories]);

	const [language, setLanguage] = useState(i18next.language);

	useEffect(() => {
		const changeLanguage = (newLang: string) => setLanguage(newLang);
		i18next.on("languageChanged", changeLanguage);
		return () => i18next.off("languageChanged", changeLanguage);
	}, []);

	const [updateState, setUpdateState] = useState<CheckForUpdateResponse | null>(
		null,
	);

	useEffect(() => {
		let cancel = false;
		(async () => {
			try {
				const checkVersion = await utilCheckForUpdate();
				if (cancel) return;
				if (checkVersion.is_update_available) {
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
				<I18nextProvider i18n={i18next}>
					<TooltipProvider>
						{updateState && (
							<CheckForUpdateMessage
								response={updateState}
								close={() => setUpdateState(null)}
							/>
						)}
						<div lang={language} className="contents">
							{children}
						</div>
					</TooltipProvider>
				</I18nextProvider>
			</QueryClientProvider>
		</>
	);
}
