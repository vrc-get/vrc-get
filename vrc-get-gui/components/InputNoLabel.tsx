import {Input} from "@/components/ui/input";
import React, {ComponentProps} from "react";

export function InputNoLabel(
	props: ComponentProps<typeof Input>
) {
	return (
		<Input
			{...props}
			className={`!border-t-blue-gray-300 placeholder:text-blue-gray-300 focus:!border-blue-gray-300 ${props.className}`}
		/>
	)
}
