import { createFileRoute, Outlet } from "@tanstack/react-router";
import ErrorPage from "@/app/-error";

export const Route = createFileRoute("/")({
	component: RouteComponent,
	errorComponent: ErrorPage,
});

function RouteComponent() {
	return (
		<>
			<Outlet />
		</>
	);
}
