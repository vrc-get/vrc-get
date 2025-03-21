import { Dialog } from "@/components/ui/dialog";
import React, {
	useEffect,
	useRef,
	useState,
	useSyncExternalStore,
} from "react";

export interface DialogContext<in R> {
	close: (arg: R) => void;
	closing: boolean;
}

type DialogProps<R> = {
	dialog: DialogContext<R>;
};

type DialogResult<P> = P extends DialogProps<infer R> ? R : unknown;

interface DialogApi<State> {
	readonly currentState: State;
	progress(state: State): void;

	ask<P extends DialogProps<never>>(
		component: React.JSXElementConstructor<P>,
		props: NoInfer<Omit<P, "dialog">>,
	): Promise<DialogResult<P>>;
	askClosing<P extends DialogProps<never>>(
		component: React.JSXElementConstructor<P>,
		props: NoInfer<Omit<P, "dialog">>,
	): Promise<DialogResult<P>>;
	close(): void;
}

function showDialog<State>(
	initialState: State,
	progressComponent: (state: State) => React.ReactElement,
): DialogApi<State> {
	if (dialogGlobalState == null) throw new Error("No Root is mounted");
	const globalState = dialogGlobalState;

	const key = globalState.getKey();
	const stateStore = new SyncStore(initialState);
	const askStore = new SyncStore<React.ReactElement | null>(null);

	function closeImpl() {
		globalState.closeDialog(key);
	}

	function askImpl<P extends DialogProps<never>>(
		component: React.JSXElementConstructor<P>,
		props: NoInfer<Omit<P, "dialog">>,
		closing: boolean,
	): Promise<DialogResult<P>> {
		if (askStore.value != null) throw new Error("another ask in progress");

		let resolve: (result: DialogResult<P>) => void;
		const promise = new Promise<DialogResult<P>>((r) => (resolve = r));

		const dialog: DialogContext<DialogResult<P>> = {
			closing: closing,
			close(r) {
				// if the dialog is NOT closing, we don't detach the
				if (closing) closeImpl();
				else askStore.value = null;
				resolve(r);
			},
		};

		askStore.value = React.createElement<P>(component, {
			...props,
			dialog,
		} as unknown as P);

		return promise;
	}

	const result: DialogApi<State> = {
		get currentState() {
			return stateStore.value;
		},
		progress(newState) {
			stateStore.value = newState;
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
	};

	globalState.openDialog(
		key,
		<DialogBodyElement
			askStore={askStore}
			stateStore={stateStore}
			progressComponent={progressComponent}
		/>,
	);

	return result;
}

export function openSingleDialog<P extends DialogProps<never>>(
	component: React.JSXElementConstructor<P>,
	props: NoInfer<Omit<P, "dialog">>,
): Promise<DialogResult<P>> {
	return showDialog(null, () => <></>).askClosing(component, props);
}

interface GlobalState {
	getKey(): number;
	openDialog(key: number, element: React.ReactElement): void;
	closeDialog(key: number): void;
}

let dialogGlobalState: GlobalState | null = null;

function DialogBodyElement<State>({
	askStore,
	stateStore,
	progressComponent,
}: {
	askStore: SyncStore<React.ReactElement | null>;
	stateStore: SyncStore<State>;
	progressComponent: (state: State) => React.ReactElement;
}) {
	const ask = askStore.use();
	const state = stateStore.use();
	if (ask != null) return ask;
	else return progressComponent(state);
}

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
