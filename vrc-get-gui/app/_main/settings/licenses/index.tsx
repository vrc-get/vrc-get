import { loadLicenses } from "@/lib/licenses";
import { createFileRoute } from "@tanstack/react-router";
import { use } from "react";
import RenderPage from "./-render-client";

const licensesPromise = loadLicenses();

export const Route = createFileRoute("/_main/settings/licenses/")({
	component: Page,
});

function Page() {
	const licenses = use(licensesPromise);
	return <RenderPage licenses={licenses} />;
}
