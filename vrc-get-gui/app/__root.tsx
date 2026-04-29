import { createRootRoute, Outlet } from "@tanstack/react-router";
import ErrorPage from "@/app/-error";
import { Providers } from "@/components/providers";
import "./globals.css";
import React, { Suspense } from "react";

const TanStackRouterDevtools = import.meta.env.PROD
	? () => null // Render nothing in production
	: React.lazy(() =>
			// Lazy load in development
			import("@tanstack/router-devtools").then((res) => ({
				default: res.TanStackRouterDevtools,
				// For Embedded Mode
				// default: res.TanStackRouterDevtoolsPanel
			})),
		);

export const Route = createRootRoute({
	component: RootComponent,
	errorComponent: ErrorPage,
});

function RootComponent() {
	return (
		<>
			<Providers>
				<Outlet />
			</Providers>
			<Suspense>
				<TanStackRouterDevtools position={"bottom-right"} />
			</Suspense>
		</>
	);
}
