import { LoaderCircle } from "lucide-react";

export default function Loading({
	loadingText = "Loading...",
}: {
	loadingText?: React.ReactNode;
}) {
	return (
		<div className="flex flex-col items-center justify-center h-full w-full space-y-4">
			<LoaderCircle className="h-10 w-10 animate-spin" />
			<p className="text-xl font-semibold text-gray-700">{loadingText}</p>
		</div>
	);
}
