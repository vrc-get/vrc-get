import {toast} from "react-toastify";

export function unsupported(feature: string): () => void {
	return () => toast.error(`${feature} is not supported yet`)
}
