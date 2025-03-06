import { loadLicenses } from "@/lib/licenses";
import { createFileRoute } from "@tanstack/react-router";
import RenderPage from "./-render-client";

const licenses = await loadLicenses();

export const Route = createFileRoute("/_main/settings/licenses/")({
	component: Page,
});

function Page() {
	return <RenderPage licenses={licenses} />;
}
