"use client"

import {QueryClient, QueryClientProvider} from "@tanstack/react-query";
import {ToastContainer} from 'react-toastify';
import {useCallback, useEffect, useState} from "react";
import {deepLinkHasAddRepository, environmentLanguage, environmentTheme, LogEntry} from "@/lib/bindings";
import i18next from "@/lib/i18n";
import {I18nextProvider} from "react-i18next";
import {toastError, toastNormal} from "@/lib/toast";
import {useTauriListen} from "@/lib/use-tauri-listen";
import {usePathname, useRouter} from "next/navigation";
import {TooltipProvider} from "@/components/ui/tooltip";

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
		(async () => {
			let theme = await environmentTheme();
			if (theme === "system") {
				const {appWindow} = await import("@tauri-apps/api/window");
				theme = await appWindow.theme() ?? "light";
			}
			document.documentElement.setAttribute("class", theme);
		})();
	}, [])

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
				theme="colored"
				className={"whitespace-normal"}
			/>
			<QueryClientProvider client={queryClient}>
				<I18nextProvider i18n={i18next}>
					<TooltipProvider>
						<div lang={language} className="contents">
							{children}
						</div>
					</TooltipProvider>
				</I18nextProvider>
			</QueryClientProvider>
		</>
	);
}
