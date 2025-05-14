import { RouterProvider, createRouter } from "@tanstack/react-router";
import ReactDOM from "react-dom/client";
import { routeTree } from "./routeTree.gen";
import "./polyfill";
import "./project-update";

// Set up a Router instance
export const router = createRouter({
	routeTree,
	defaultPreload: "intent",
});

// Register things for typesafety
declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}

// biome-ignore lint/style/noNonNullAssertion: known to be non-null
const rootElement = document.getElementById("app")!;

if (!rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(<RouterProvider router={router} />);
}
