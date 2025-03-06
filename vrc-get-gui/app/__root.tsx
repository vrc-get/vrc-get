import ErrorPage from "@/app/-error";
import { Providers } from "@/components/providers";
import { Outlet, createRootRoute } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/router-devtools";
import "./globals.css";

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
			<TanStackRouterDevtools position="bottom-right" />
		</>
	);
}
