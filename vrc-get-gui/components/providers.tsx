"use client"

import {QueryClient, QueryClientProvider} from "@tanstack/react-query";
import {ToastContainer} from 'react-toastify';
import {useCallback, useEffect} from "react";
import {environmentLanguage, LogEntry} from "@/lib/bindings";
import i18next from "@/lib/i18n";
import {I18nextProvider} from "react-i18next";
import {toastError} from "@/lib/toast";
import {ThemeProvider} from "@material-tailwind/react";
import {useTauriListen} from "@/lib/use-tauri-listen";

const queryClient = new QueryClient();

export function Providers({children}: { children: React.ReactNode }) {
	useTauriListen<LogEntry>("log", useCallback((event) => {
		const entry = event.payload as LogEntry;
		if (entry.level === "Error") {
			toastError(entry.message);
		}
	}, []))

	useEffect(() => {
		environmentLanguage().then((lang) => i18next.changeLanguage(lang))
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
					<ThemeProvider value={{
						Typography: {
							styles: {
								font: 'normal'
							}
						}
					}}>
						{children as any}
					</ThemeProvider>
				</I18nextProvider>
			</QueryClientProvider>
		</>
	);
}
