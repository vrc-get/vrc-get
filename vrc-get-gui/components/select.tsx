// based on https://github.com/creativetimofficial/material-tailwind/blob/main/packages/material-tailwind-react/src/components/Select/index.tsx#L298

import React, {createContext, useContext, useState} from "react";
import {Button} from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {useTheme} from "@material-tailwind/react";
import findMatch from "@material-tailwind/react/utils/findMatch";
import objectsToString from "@material-tailwind/react/utils/objectsToString";
import {twMerge} from "tailwind-merge";
import classnames from "classnames";
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

	const {select} = useTheme();
	const {defaultProps, valid, styles} = select;
	const {base, variants} = styles;

	const contextValue: SelectContext = {
		onClick(value: any) {
			onChange?.(value);
			setOpen(false);
		}
	}

	const size = defaultProps.size;

	const selectVariant = variants.outlined;
	const selectSize = selectVariant.sizes[findMatch(valid.sizes, size, "md")];
	const stateClasses = selectVariant.states[state];
	const containerClasses = classnames(
		objectsToString(base.container),
		objectsToString(selectSize.container),
	);
	const selectClasses = twMerge(
		classnames(
			objectsToString(base.select),
			objectsToString(selectVariant.base.select),
			objectsToString(stateClasses.select),
			objectsToString(selectSize.select),
		),
		className,
	);
	const arrowClasses = classnames(objectsToString(base.arrow.initial), {
		[objectsToString(base.arrow.active)]: open,
	});
	const buttonContentClasses = "absolute top-2/4 -translate-y-2/4 left-3 pt-0.5";

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
				<div className={containerClasses}>
					<DropdownMenuTrigger asChild className={"lowercase"}>
						<Button className={selectClasses} disabled={disabled}>
							<span className={buttonContentClasses}>{value}</span>
							<div className={arrowClasses}>
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
