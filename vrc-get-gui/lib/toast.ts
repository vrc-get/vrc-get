import React from "react";
import { type ToastContent, toast } from "react-toastify";
import type { RustError } from "./bindings";
import { tc } from "./i18n";

function wrapWithDiv(content: ToastContent): ToastContent {
	if (typeof content === "function") return content;
	return React.createElement("div", {}, content);
}

export function toastNormal(message: ToastContent) {
	toast(wrapWithDiv(message), {
		pauseOnFocusLoss: false,
	});
}

export function toastInfo(message: ToastContent) {
	toast.info(wrapWithDiv(message), {
		pauseOnFocusLoss: false,
	});
}

export function toastSuccess(message: ToastContent) {
	toast.success(wrapWithDiv(message), {
		pauseOnFocusLoss: false,
	});
}

export function toastError(message: ToastContent) {
	toast.error(wrapWithDiv(message));
}

export function toastThrownError(error: unknown) {
	switch (typeof error) {
		case "string":
			toastError(error);
			break;
		case "object":
			if (error === null) return;
			if ("type" in error && error.type === "Unrecoverable") return; // should be handled by log toast
			if ("type" in error && error.type === "Localizable") {
				const e = error as RustError & { type: "Localizable" };
				toastError(tc(e.id, e.args));
				return;
			}
			if (error instanceof Error) {
				toastError(error.message);
			} else if ("message" in error && typeof error.message === "string") {
				// some non-Error errors like Handleable errors from rust
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
