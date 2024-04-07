import {Input} from "@material-tailwind/react";
import React, {ComponentProps} from "react";

export function InputNoLabel(
	props: ComponentProps<typeof Input>
) {
	return (
		<Input
			{...props}
			containerProps={{
				...props.containerProps,
				className: `min-w-[100px] ${props.containerProps?.className}`,
			}}
			className={`!border-t-blue-gray-300 placeholder:text-blue-gray-300 focus:!border-blue-gray-300 ${props.className}`}
			labelProps={{
				...props.labelProps,
				className: `before:content-none after:content-none ${props.labelProps?.className}`
			}}
		/>
	)
}
