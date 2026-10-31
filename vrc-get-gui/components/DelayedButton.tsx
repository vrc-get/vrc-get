/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import type React from "react";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
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
