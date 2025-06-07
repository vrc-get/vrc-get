import { tc } from "@/lib/i18n";
import type React from "react";

export function dateToString(dateIn: Date | number | string) {
	const date = typeof dateIn !== "object" ? new Date(dateIn) : dateIn;

	const year = date.getFullYear().toString().padStart(4, "0");
	const month = (date.getMonth() + 1).toString().padStart(2, "0");
	const day = date.getDate().toString().padStart(2, "0");
	const hours = date.getHours().toString().padStart(2, "0");
	const minutes = date.getMinutes().toString().padStart(2, "0");
	const seconds = date.getSeconds().toString().padStart(2, "0");

	return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
}

export function formatDateOffset(
	dateIn: Date | number | string,
): React.ReactNode {
	const date =
		typeof dateIn === "object"
			? dateIn.getTime()
			: typeof dateIn === "string"
				? new Date(dateIn).getTime()
				: dateIn;

	const now = Date.now();
	const diff = now - date;

	const PER_SECOND = 1000;
	const PER_MINUTE = 60 * PER_SECOND;
	const PER_HOUR = 60 * PER_MINUTE;
	const PER_DAY = 24 * PER_HOUR;
	const PER_WEEK = 7 * PER_DAY;
	const PER_MONTH = 30 * PER_DAY;
	const PER_YEAR = 365 * PER_DAY;

	const diffAbs = Math.abs(diff);

	if (diffAbs < PER_MINUTE) return tc("projects:last modified:moments");
	if (diffAbs < PER_HOUR)
		return tc("projects:last modified:minutes", {
			count: Math.floor(diff / PER_MINUTE),
		});
	if (diffAbs < PER_DAY)
		return tc("projects:last modified:hours", {
			count: Math.floor(diff / PER_HOUR),
		});
	if (diffAbs < PER_WEEK)
		return tc("projects:last modified:days", {
			count: Math.floor(diff / PER_DAY),
		});
	if (diffAbs < PER_MONTH)
		return tc("projects:last modified:weeks", {
			count: Math.floor(diff / PER_WEEK),
		});
	if (diffAbs < PER_YEAR)
		return tc("projects:last modified:months", {
			count: Math.floor(diff / PER_MONTH),
		});

	return tc("projects:last modified:years", {
		count: Math.floor(diff / PER_YEAR),
	});
}
