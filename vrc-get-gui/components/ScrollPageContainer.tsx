import {ScrollArea} from "@/components/ui/scroll-area";

export function ScrollPageContainer({ children }: { children: React.ReactNode }) {
	return (
		<ScrollArea className={"-mr-2.5 pr-2.5"} scrollBarClassName={"bg-background rounded-full border-l-0 p-[1.5px]"}>
			{children}
		</ScrollArea>
	);
}
