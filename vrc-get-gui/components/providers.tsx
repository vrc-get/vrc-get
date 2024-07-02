"use client"

import {QueryClient, QueryClientProvider} from "@tanstack/react-query";
import {ToastContainer} from 'react-toastify';
import {useCallback, useEffect, useState} from "react";
import {
	CheckForUpdateResponse,
	deepLinkHasAddRepository,
	environmentLanguage,
	environmentTheme,
	LogEntry,
	utilCheckForUpdate
} from "@/lib/bindings";
import i18next, {tc} from "@/lib/i18n";
import {I18nextProvider} from "react-i18next";
import {toastError, toastNormal, toastThrownError} from "@/lib/toast";
import {useTauriListen} from "@/lib/use-tauri-listen";
import {usePathname, useRouter} from "next/navigation";
import {Tooltip, TooltipContent, TooltipProvider} from "@/components/ui/tooltip";
import {Dialog, DialogContent} from "@/components/ui/dialog";
import {CheckForUpdateMessage} from "@/components/CheckForUpdateMessage";

const queryClient = new QueryClient();

export function Providers({children}: { children: React.ReactNode }) {
	const router = useRouter();
	const pathname = usePathname();

	useTauriListen<LogEntry>("log", useCallback((event) => {
		const entry = event.payload as LogEntry;
		if (entry.level === "Error") {
			toastError(entry.message);
		}
	}, []))

	const moveToRepositories = useCallback(() => {
		if (location.pathname != "/repositories") {
			router.push("/repositories");
		}
	}, [router]);

	useTauriListen<null>("deep-link-add-repository", useCallback((_) => {
		moveToRepositories();
	}, [moveToRepositories]));

	useEffect(() => {
		let cancel = false;
		deepLinkHasAddRepository().then((has) => {
			if (cancel) return;
			if (has) {
				moveToRepositories();
			}
		})
		return () => {
			cancel = true;
		}
	}, [moveToRepositories]);

	useEffect(() => {
		environmentLanguage().then((lang) => i18next.changeLanguage(lang))
	}, []);

	const [language, setLanguage] = useState(i18next.language);

	useEffect(() => {
		const changeLanguage = (newLang: string) => setLanguage(newLang);
		i18next.on("languageChanged", changeLanguage);
		return () => i18next.off("languageChanged", changeLanguage);
	}, []);

	useEffect(() => {
		// initially set theme based on query parameter for early feedback
		if ('location' in globalThis) {
			const search = new URLSearchParams(location.search);
			let theme = search.get('theme');
			if (theme) {
				if (theme === "system") {
					const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
					theme = isDark ? "dark" : "light";
				}
				document.documentElement.setAttribute("class", theme);
			}
		}

		(async () => {
			// then, load theme from environment
			// the theme can be different from the query parameter if the user has changed it in the settings
			let theme = await environmentTheme();
			if (theme === "system") {
				const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
				theme = isDark ? "dark" : "light";
			}
			document.documentElement.setAttribute("class", theme);
		})();
	}, [])

	const [updateState, setUpdateState] = useState<CheckForUpdateResponse | null>(null);

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
				toastThrownError(e)
				console.error(e)
			}
		})()
		return () => {
			cancel = true;
		}
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
						{updateState && <CheckForUpdateMessage response={updateState} close={() => setUpdateState(null)}/>}
						<div lang={language} className="contents">
							{children}
						</div>
					</TooltipProvider>
				</I18nextProvider>
			</QueryClientProvider>
		</>
	);
}
