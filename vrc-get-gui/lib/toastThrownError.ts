import {toast} from "react-toastify";

export function toastThrownError(error: any) {
	if ('Unrecoverable' in error) return; // should be handled by log toast
	toast.error(error.message);
}
