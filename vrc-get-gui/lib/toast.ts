import {toast} from "react-toastify";

export function toastNormal(message: string) {
	toast(message, {
		pauseOnFocusLoss: false,
	});
}

export function toastSuccess(message: string) {
	toast.success(message, {
		pauseOnFocusLoss: false,
	});
}

export function toastError(message: string) {
	toast.error(message);
}

export function toastThrownError(error: any) {
	if ('Unrecoverable' in error) return; // should be handled by log toast
	toastError(error.message);
}
