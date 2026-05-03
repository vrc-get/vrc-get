let operationInProgressCount = 0;

export interface OperationScope {
	finish(): void;
	[Symbol.dispose](): void;
}

export function beginOperation(): OperationScope {
	let finished = false;
	operationInProgressCount++;
	return {
		finish() {
			if (!finished) {
				finished = true;
				operationInProgressCount--;
				if (operationInProgressCount < 0) operationInProgressCount = 0;
			}
		},
		[Symbol.dispose]() {
			this.finish();
		},
	};
}

export function isOperationInProgress(): boolean {
	return operationInProgressCount > 0;
}
