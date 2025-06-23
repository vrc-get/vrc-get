import { ArrowDown, ArrowUp, CircleMinus, CirclePlus } from "lucide-react";
import type React from "react";
import {
	type Dispatch,
	type SetStateAction,
	useCallback,
	useMemo,
	useState,
} from "react";
import { Button } from "@/components/ui/button";
import { assertNever } from "@/lib/assert-never";

const internalSymbol: unique symbol = Symbol("ReorderableListContextInternal");
const idSymbol: unique symbol = Symbol("IdSymbol");

type Id = { [idSymbol]: number; toString: () => string };

type NonFunction =
	| string
	| number
	| boolean
	| null
	| undefined
	| symbol
	| bigint
	| object;

type AddOptions = { after: Id } | { before: Id };

type ReordeableListValue<T> = { id: Id; value: T };

type ReorderableListContextInternal<T> = {
	backedList: ReordeableListValue<T>[];
	defaultValue: T;
	reorderable: boolean;
	addable: boolean;
	swap: (index1: number, index2: number) => void;
};

export type ReorderableListContext<T> = {
	setList: Dispatch<SetStateAction<T[]>>;
	add: (value: T, options?: AddOptions) => void;
	remove: (id: Id) => void;
	update: (id: Id, action: SetStateAction<T>) => void;
	get value(): T[];
	[internalSymbol]: ReorderableListContextInternal<T>;
};

let globalId = 0;

function makeValue<T>(value: T): ReordeableListValue<T> {
	const idNumber = globalId++;
	return {
		id: {
			[idSymbol]: idNumber,
			toString: () => `id-${idNumber}`,
		},
		value,
	};
}

export function useReorderableList<T extends NonFunction>({
	defaultValue,
	defaultArray,
	allowEmpty = true,
	reorderable = true,
	addable = true,
}: {
	defaultValue: T;
	defaultArray?: T[] | (() => T[]);
	allowEmpty?: boolean;
	reorderable?: boolean;
	addable?: boolean;
}): ReorderableListContext<T> {
	const [backedList, setBackedList] = useState<ReordeableListValue<T>[]>(() => {
		if (defaultArray != null) {
			let defaultSpecified =
				typeof defaultArray === "function" ? defaultArray() : defaultArray;
			if (!allowEmpty && defaultSpecified.length === 0)
				defaultSpecified = [defaultValue];
			return defaultSpecified.map(makeValue);
		} else {
			return allowEmpty ? [] : [makeValue(defaultValue)];
		}
	});

	const setList = useCallback(
		(value: SetStateAction<T[]>) => {
			if (typeof value === "function") {
				setBackedList((oldValue) => {
					let newValue = value(oldValue.map(({ value }) => value)).map(
						(value) => makeValue(value),
					);
					if (newValue.length === 0 && !allowEmpty)
						newValue = [makeValue(defaultValue)];
					return newValue;
				});
			} else {
				setBackedList(value.map(makeValue));
			}
		},
		[allowEmpty, defaultValue],
	);

	const add = useCallback((value: T, options?: AddOptions) => {
		if (options == null) {
			setBackedList((old) => [...old, makeValue(value)]);
		} else if ("after" in options) {
			setBackedList((old) => {
				const idx = old.findIndex(({ id }) => id === options.after);
				if (idx === -1) return old;
				return [
					...old.slice(0, idx + 1),
					makeValue(value),
					...old.slice(idx + 1),
				];
			});
		} else if ("before" in options) {
			setBackedList((old) => {
				const idx = old.findIndex(({ id }) => id === options.before);
				if (idx === -1) return old;
				return [...old.slice(0, idx), makeValue(value), ...old.slice(idx)];
			});
		} else {
			assertNever(options);
		}
	}, []);

	const remove = useCallback(
		(id: Id) => {
			setBackedList((old) => {
				let list = old.filter(({ id: _id }) => _id !== id);
				if (list.length === 0 && !allowEmpty) list = [makeValue(defaultValue)];
				return list;
			});
		},
		[allowEmpty, defaultValue],
	);

	const update = useCallback((id: Id, action: SetStateAction<T>) => {
		if (typeof action === "function") {
			setBackedList((old) => {
				const idx = old.findIndex(({ id: _id }) => _id === id);
				if (idx === -1) return old;
				const newValue = action(old[idx].value);
				const newArray = [...old];
				newArray[idx] = { id, value: newValue };
				return newArray;
			});
		} else {
			setBackedList((old) => {
				const idx = old.findIndex(({ id: _id }) => _id === id);
				if (idx === -1) return old;
				const newArray = [...old];
				newArray[idx] = { id, value: action };
				return newArray;
			});
		}
	}, []);

	const swap = useCallback((index1: number, index2: number) => {
		setBackedList((old) => {
			const newArray = [...old];
			const tmp = newArray[index1];
			newArray[index1] = newArray[index2];
			newArray[index2] = tmp;
			return newArray;
		});
	}, []);

	return useMemo(
		() => ({
			setList,
			add,
			update,
			remove,
			get value() {
				return backedList.map(({ value }) => value);
			},
			[internalSymbol]: {
				backedList,
				defaultValue,
				swap,
				reorderable,
				addable,
			},
		}),
		[
			setList,
			add,
			update,
			remove,
			backedList,
			defaultValue,
			swap,
			reorderable,
			addable,
		],
	);
}

export function ReorderableList<T>({
	context,
	renderItem,
	ifEmpty,
	disabled,
}: {
	context: ReorderableListContext<T>;
	renderItem: (value: T, id: Id) => React.ReactNode;
	ifEmpty?: () => React.ReactNode;
	disabled?: boolean;
}) {
	const internal = context[internalSymbol];

	if (internal.backedList.length === 0) {
		return ifEmpty?.();
	}

	return internal.backedList.map(({ value, id }, i) => (
		<tr key={id[idSymbol]}>
			{renderItem(value, id)}
			<td className={"w-1"}>
				<div className={"flex flex-row ml-1.5"}>
					{internal.addable && (
						<Button
							disabled={disabled}
							variant={"ghost"}
							size={"icon"}
							onClick={() => context.add(internal.defaultValue, { after: id })}
						>
							<CirclePlus color={"green"} className={"size-5"} />
						</Button>
					)}
					<Button
						disabled={disabled}
						variant={"ghost"}
						size={"icon"}
						onClick={() => context.remove(id)}
					>
						<CircleMinus color={"red"} className={"size-5"} />
					</Button>
					{internal.reorderable && (
						<div className={"flex flex-col w-10 align-middle"}>
							<Button
								disabled={disabled || i === 0}
								variant={"ghost"}
								size={"icon"}
								className={"h-5"}
								onClick={() => internal.swap(i, i - 1)}
							>
								<ArrowUp className={"size-2.5"} />
							</Button>
							<Button
								disabled={disabled || i === internal.backedList.length - 1}
								variant={"ghost"}
								size={"icon"}
								className={"h-5"}
								onClick={() => internal.swap(i, i + 1)}
							>
								<ArrowDown className={"size-2.5"} />
							</Button>
						</div>
					)}
				</div>
			</td>
		</tr>
	));
}
