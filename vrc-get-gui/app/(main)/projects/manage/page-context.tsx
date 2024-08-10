import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	type ComponentProps,
	type ElementRef,
	createContext,
	forwardRef,
	useContext,
} from "react";

interface PageContext {
	isLoading: boolean;
}

export const PageContext = createContext<PageContext>({
	isLoading: false,
});
PageContext.displayName = "PageContext";

export const PageContextProvider = PageContext.Provider;

export function usePageContext() {
	return useContext(PageContext);
}

export const ButtonDisabledIfLoading = forwardRef<
	ElementRef<typeof Button>,
	ComponentProps<typeof Button>
>(function ButtonDisabledIfLoading({ disabled, ...props }, ref) {
	const { isLoading } = usePageContext();
	return <Button disabled={isLoading || disabled} {...props} ref={ref} />;
});

export const CheckboxDisabledIfLoading = forwardRef<
	ElementRef<typeof Checkbox>,
	ComponentProps<typeof Checkbox>
>(function CheckboxDisabledIfLoading({ disabled, ...props }, ref) {
	const { isLoading } = usePageContext();
	return <Checkbox disabled={isLoading || disabled} {...props} ref={ref} />;
});
