import {toast, ToastContent} from "react-toastify";

export function toastNormal(message: ToastContent) {
	toast(message, {
		pauseOnFocusLoss: false,
	});
}

export function toastInfo(message: ToastContent) {
	toast.info(message, {
		pauseOnFocusLoss: false,
	});
}

export function toastSuccess(message: ToastContent) {
	toast.success(message, {
		pauseOnFocusLoss: false,
	});
}

export function toastError(message: ToastContent) {
	toast.error(message);
}

export function toastThrownError(error: any) {
	switch (typeof error) {
		case 'string':
			toastError(error);
			break;
		case 'object':
			if ('type' in error && error.type === "Unrecoverable") return; // should be handled by log toast
			if (error instanceof Error || 'message' in error) {
				toastError(error.message);
			} else {
				toastError(JSON.stringify(error));
			}
			break;
		default:
			toastError(JSON.stringify(error));
			break;
	}
}
