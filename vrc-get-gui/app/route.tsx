import ErrorPage from "@/app/-error";
import { Outlet, createFileRoute } from "@tanstack/react-router";

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
