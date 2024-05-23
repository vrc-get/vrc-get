import {Input} from "@/components/ui/input";
import React, {ComponentProps} from "react";

export function InputNoLabel(
	props: ComponentProps<typeof Input>
) {
	return (
		<Input
			{...props}
			className={`border-t-primary placeholder:text-primary focus:border-primary ${props.className}`}
		/>
	)
}
