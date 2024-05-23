// based on https://github.com/creativetimofficial/material-tailwind/blob/main/packages/material-tailwind-react/src/components/Select/index.tsx#L298

import React, {createContext, useContext, useState} from "react";
import {Button} from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {ChevronDownIcon} from "@heroicons/react/24/solid";

interface SelectContext {
	onClick(value: any): void;
}

export const SelectContext = createContext<SelectContext | undefined>(undefined);

export function VGSelect(
	{
		children,
		disabled,
		value,
		className,
		menuClassName,
		onChange,
	}: {
		children: React.ReactNode,
		disabled?: boolean,
		value?: React.ReactNode,
		className?: string,
		menuClassName?: string,
		onChange?: (value: any) => void,
	}
) {
	const [state, setState] = useState<string>("close");
	const [open, setOpen] = React.useState(false);

	const contextValue: SelectContext = {
		onClick(value: any) {
			onChange?.(value);
			setOpen(false);
		}
	}

	React.useEffect(() => {
		if (open) {
			setState("open");
		} else {
			setState("close");
		}
	}, [open, value]);

	return (
		<SelectContext.Provider value={contextValue}>
			<DropdownMenu open={open} onOpenChange={() => setOpen(!open)}>
				<div className={"relative w-full min-w-[200px] h-10"}>
					<DropdownMenuTrigger asChild>
						<Button variant={"outline"} className={`lowercase w-full ${className}`} disabled={disabled}>
							<span className={"text-muted-foreground absolute top-2/4 -translate-y-2/4 left-3 pt-0.5"}>{value}</span>
							<div className={"grid place-items-center absolute right-2 w-5 text-info"}>
								<ChevronDownIcon className="size-3"/>
							</div>
						</Button>
					</DropdownMenuTrigger>
				</div>
				<DropdownMenuContent className={`max-h-96 overflow-y-scroll ${menuClassName}`}>
					{children}
				</DropdownMenuContent>
			</DropdownMenu>
		</SelectContext.Provider>
	)
}

export const VGOption = React.forwardRef(VGOptionImpl)

function VGOptionImpl(
	{
		children,
		value,
		disabled,
	}: {
		children: React.ReactNode,
		value: any,
		disabled?: boolean,
	},
	ref: React.Ref<HTMLDivElement>
) {
	const contextValue = useContext(SelectContext);
	return (
		<DropdownMenuItem ref={ref} disabled={disabled} onClick={() => contextValue?.onClick(value)}>{children}</DropdownMenuItem>
	)
}
