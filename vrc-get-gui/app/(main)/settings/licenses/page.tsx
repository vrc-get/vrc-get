import {loadLicenses} from "@/lib/licenses";
import RenderPage from "./render-client";

const licenses = await loadLicenses();

export default function Page() {
	return <RenderPage licenses={licenses}/>;
}

