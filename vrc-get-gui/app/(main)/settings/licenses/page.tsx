import { loadLicenses } from "@/lib/licenses";
import RenderPage from "./render-client";
import Loading from "@/app/loading";
import React, {Suspense} from "react";

const licenses = await loadLicenses();

export default function Page() {
	return <Suspense fallback={<Loading />}><RenderPage licenses={licenses} /></Suspense>;
}
