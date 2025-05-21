import { Button } from "@/components/ui/button";
import type React from "react";
import { useEffect, useRef, useState } from "react";
export function DelayedButton({
	disabled,
	delay,
	...props
}: React.ComponentProps<typeof Button> & {
	delay: number;
}) {
	const delayRef = useRef<number>(delay);
	const delayTimer = useRef<number>(null);
	const [delayPassed, setDelayPassed] = useState(false);
	delayRef.current = delay;

	useEffect(() => {
		if (delayTimer.current != null) clearTimeout(delayTimer.current);
		if (!disabled) {
			if (delayRef.current === 0) {
				setDelayPassed(true);
			} else {
				setDelayPassed(false);
				delayTimer.current = window.setTimeout(
					() => setDelayPassed(true),
					delayRef.current,
				);
			}
		} else {
			setDelayPassed(false);
		}
	}, [disabled]);

	return <Button {...props} disabled={!delayPassed || disabled} />;
}
