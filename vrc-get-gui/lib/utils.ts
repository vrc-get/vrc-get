import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function comparator<T>(a: T, b: T): -1 | 0 | 1 {
	return a < b ? -1 : a > b ? 1 : 0;
}

export function keyComparator<T>(key: keyof T): (a: T, b: T) => -1 | 0 | 1 {
	return (a, b) => comparator(a[key], b[key]);
}

export function groupBy<T, K>(values: T[], key: (value: T) => K): Map<K, T[]> {
	const map = new Map<K, T[]>();
	for (const value of values) {
		const keyValue = key(value);
		let list = map.get(keyValue);
		if (list == null) map.set(keyValue, (list = []));
		list.push(value);
	}
	return map;
}
