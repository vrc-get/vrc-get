import Loading from "@/app/loading";
import { loadLicenses } from "@/lib/licenses";
import { Suspense } from "react";
import RenderPage from "./render-client";

const licenses = await loadLicenses();

export default function Page() {
	return (
		<Suspense fallback={<Loading />}>
			<RenderPage licenses={licenses} />
		</Suspense>
	);
}
