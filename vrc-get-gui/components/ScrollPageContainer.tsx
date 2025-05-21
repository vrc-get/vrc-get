import { ScrollArea } from "@/components/ui/scroll-area";

export function ScrollPageContainer({
	children,
	className,
	viewportClassName,
}: {
	children: React.ReactNode;
	className?: string;
	viewportClassName?: string;
}) {
	return (
		<ScrollArea
			className={`-mr-3 pr-3 ${className}`}
			scrollBarClassName={"bg-background rounded-full border-l-0 p-[1.5px]"}
			viewportClassName={`${viewportClassName}`}
		>
			{children}
		</ScrollArea>
	);
}
