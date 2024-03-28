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
	switch (typeof error) {
		case 'string':
			toastError(error);
			break;
		case 'object':
			if ('Unrecoverable' in error) return; // should be handled by log toast
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
