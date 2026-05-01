let operationInProgressCount = 0;

export function beginOperation(): void {
	operationInProgressCount++;
}

export function endOperation(): void {
	operationInProgressCount--;
	if (operationInProgressCount < 0) operationInProgressCount = 0;
}

export function isOperationInProgress(): boolean {
	return operationInProgressCount > 0;
}
