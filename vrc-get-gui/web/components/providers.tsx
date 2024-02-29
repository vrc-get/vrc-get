"use client"

import {QueryClient, QueryClientProvider} from "@tanstack/react-query";
import {ToastContainer} from 'react-toastify';

const queryClient = new QueryClient();

export function Providers({children}: { children: React.ReactNode }) {
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
