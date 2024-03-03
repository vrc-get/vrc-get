"use client"

import {QueryClient, QueryClientProvider} from "@tanstack/react-query";
import {toast, ToastContainer} from 'react-toastify';
import {useEffect} from "react";
import {listen} from "@tauri-apps/api/event";
import {LogEntry} from "@/lib/bindings";

const queryClient = new QueryClient();

export function Providers({children}: { children: React.ReactNode }) {
	useEffect(() => {
		let unlisten: (() => void) | undefined = undefined;
		let unlistened = false;

		listen("log", (event) => {
			const entry = event.payload as LogEntry;
			if (entry.level === "Error") {
				toast.error(entry.message);
			}
		}).then((unlistenFn) => {
			if (unlistened) {
				unlistenFn();
			} else {
				unlisten = unlistenFn;
			}
		});

		return () => {
			unlisten?.();
			unlistened = true;
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
				{children}
			</QueryClientProvider>
		</>
	);
}
