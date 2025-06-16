import { Dialog, DialogContent } from "@/components/ui/dialog";
import { assertNever } from "@/lib/assert-never";
import React, {
	useEffect,
	useRef,
	useState,
	useSyncExternalStore,
} from "react";

export interface DialogContext<in R> {
	close: (arg: R) => void;
	error: (arg: unknown) => void;
	closing: boolean;
}

type DialogProps<R> = {
	dialog: DialogContext<R>;
};

type DialogResult<P> = P extends DialogProps<infer R> ? R : unknown;

export interface DialogApi {
	replace(state: React.ReactElement): void;

	ask<P extends DialogProps<never>>(
		component: React.JSXElementConstructor<P>,
		props: NoInfer<Omit<P, "dialog">>,
	): Promise<DialogResult<P>>;
	askClosing<P extends DialogProps<never>>(
		component: React.JSXElementConstructor<P>,
		props: NoInfer<Omit<P, "dialog">>,
	): Promise<DialogResult<P>>;
	close(): void;
	[Symbol.dispose](): void;
}

type DialogState =
	| {
			type: "before";
	  }
	| {
			type: "asking";
			element: React.ReactElement;
	  }
	| {
			type: "asked";
			element: React.ReactElement;
	  }
	| {
			type: "content";
			element: React.ReactElement;
	  };

export function showDialog(
	initialContent: React.ReactElement | null = null,
): DialogApi {
	if (dialogGlobalState == null) throw new Error("No Root is mounted");
	const globalState = dialogGlobalState;

	const key = globalState.getKey();
	const dialogState = new SyncStore<DialogState>(
		initialContent == null
			? { type: "before" }
			: { type: "content", element: initialContent },
	);
	let dialogOpened = false;

	function closeImpl() {
		if (dialogOpened) globalState.closeDialog(key);
	}

	function askImpl<P extends DialogProps<never>>(
		component: React.JSXElementConstructor<P>,
		props: NoInfer<Omit<P, "dialog">>,
		closing: boolean,
	): Promise<DialogResult<P>> {
		if (dialogState.value.type === "asking")
			throw new Error("another ask in progress");

		let resolve: (result: DialogResult<P>) => void;
		let reject: (error: unknown) => void;
		const promise = new Promise<DialogResult<P>>((r, j) => {
			resolve = r;
			reject = j;
		});

		const dialog: DialogContext<DialogResult<P>> = {
			closing: closing,
			close(r) {
				// if the dialog is NOT closing, we don't detach the
				if (closing) closeImpl();
				else dialogState.value = { type: "asked", element };
				resolve(r);
			},
			error(e) {
				// if the dialog is NOT closing, we don't detach the
				if (closing) closeImpl();
				else dialogState.value = { type: "asked", element };
				reject(e);
			},
		};

		const element = React.createElement<P>(component, {
			...props,
			dialog,
		} as unknown as P);

		dialogState.value = { type: "asking", element };
		mayOpenDialog();

		return promise;
	}

	const result: DialogApi = {
		replace(element) {
			if (dialogState.value.type === "asking")
				throw new Error("another ask in progress");
			dialogState.value = { type: "content", element };
			mayOpenDialog();
		},
		ask<P extends DialogProps<never>>(
			component: React.JSXElementConstructor<P>,
			props: NoInfer<Omit<P, "dialog">>,
		): Promise<DialogResult<P>> {
			return askImpl(component, props, false);
		},
		askClosing<P extends DialogProps<never>>(
			component: React.JSXElementConstructor<P>,
			props: NoInfer<Omit<P, "dialog">>,
		): Promise<DialogResult<P>> {
			return askImpl(component, props, true);
		},
		close() {
			closeImpl();
		},
		[Symbol.dispose]: closeImpl,
	};

	function mayOpenDialog() {
		if (!dialogOpened)
			globalState.openDialog(
				key,
				<DialogBodyElement dialogState={dialogState} />,
			);
		dialogOpened = true;
	}
	if (dialogState.value.type !== "before") mayOpenDialog();

	return result;
}

function DialogBodyElement({
	dialogState,
}: {
	dialogState: SyncStore<DialogState>;
}) {
	const state = dialogState.use();
	const className = "max-h-[calc(100dvh-(var(--spacing)*8))] overflow-y-auto";
	switch (state.type) {
		case "before":
			return null;
		case "asking":
			return (
				<DialogContent className={className}>{state.element}</DialogContent>
			);
		case "asked":
			return (
				<DialogContent className={`${className} pointer-events-none`}>
					{state.element}
				</DialogContent>
			);
		case "content":
			return (
				<DialogContent className={className}>{state.element}</DialogContent>
			);
		default:
			assertNever(state);
	}
}

export function openSingleDialog<P extends DialogProps<never>>(
	component: React.JSXElementConstructor<P>,
	props: NoInfer<Omit<P, "dialog">>,
): Promise<DialogResult<P>> {
	return showDialog().askClosing(component, props);
}

interface GlobalState {
	getKey(): number;
	openDialog(key: number, element: React.ReactElement): void;
	closeDialog(key: number): void;
}

let dialogGlobalState: GlobalState | null = null;

const closeDelayMs = 2000;

export function DialogRoot() {
	const keyRef = useRef(0);

	interface ElementState {
		key: number;
		closing: boolean;
		element: React.ReactElement;
	}
	const [state, setState] = useState<ElementState[]>([]);

	useEffect(() => {
		if (dialogGlobalState != null)
			throw new Error("Multiple DialogRoot is mounted");
		dialogGlobalState = {
			getKey(): number {
				return keyRef.current++;
			},
			openDialog(key: number, element: React.ReactElement) {
				setState((ary) => [...ary, { key, element, closing: false }]);
			},
			closeDialog(key: number) {
				if (closeDelayMs < 0) {
					setState((ary) => ary.filter((x) => x.key !== key));
				} else {
					setState((ary) =>
						ary.map((x) => (x.key !== key ? x : { ...x, closing: true })),
					);
					setTimeout(() => {
						setState((ary) => ary.filter((x) => x.key !== key));
					}, closeDelayMs);
				}
			},
		};

		return () => {
			dialogGlobalState = null;
		};
	}, []);

	return state.map(({ closing, key, element }) => {
		return (
			<Dialog open={!closing} key={key}>
				{element}
			</Dialog>
		);
	});
}

class SyncStore<T> {
	private _value: T;
	private listeners: (() => void)[] = [];

	constructor(value: T) {
		this._value = value;
		this.getSnapshot = this.getSnapshot.bind(this);
		this.subscribe = this.subscribe.bind(this);
	}

	private getSnapshot() {
		return this.value;
	}

	private subscribe(onStoreChange: () => void): () => void {
		this.listeners.push(onStoreChange);
		return () => {
			this.listeners = this.listeners.filter((x) => x !== onStoreChange);
		};
	}

	public use() {
		return useSyncExternalStore(this.subscribe, this.getSnapshot);
	}

	public get value() {
		return this._value;
	}

	public set value(v: T) {
		this._value = v;
		for (const f of this.listeners) {
			f();
		}
	}
}
