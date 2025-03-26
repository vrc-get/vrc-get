import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { commands } from "@/lib/bindings";
import { useGlobalInfo } from "@/lib/global-info";
import i18next, { languages, tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { CircleAlert } from "lucide-react";
import type React from "react";

const environmentGetSettings = queryOptions({
	queryKey: ["environmentGetSettings"],
	queryFn: commands.environmentGetSettings,
});

const environmentLanguage = queryOptions({
	queryKey: ["environmentLanguage"],
	queryFn: commands.environmentLanguage,
});

export function LanguageSelector() {
	const queryClient = useQueryClient();
	const { data: lang } = useQuery(environmentLanguage);
	const changeLanguage = useMutation({
		mutationFn: async (language: string) =>
			await commands.environmentSetLanguage(language),
		onMutate: async (language) => {
			await i18next.changeLanguage(language);
			await queryClient.invalidateQueries(environmentLanguage);
			const data = queryClient.getQueryData(environmentLanguage.queryKey);
			queryClient.setQueryData(environmentLanguage.queryKey, language);
			return data;
		},
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
		onSettled: () => queryClient.invalidateQueries(environmentLanguage),
	});

	return (
		<label className="flex items-center">
			<span className="text-lg">
				{tc("settings:language")}
				{": "}
			</span>
			<Select value={lang} onValueChange={changeLanguage.mutate}>
				<SelectTrigger>
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						{languages.map((lang) => (
							<SelectItem key={lang} value={lang}>
								{tc("settings:langName", { lng: lang })}
							</SelectItem>
						))}
					</SelectGroup>
				</SelectContent>
			</Select>
		</label>
	);
}

const environmentTheme = queryOptions({
	queryKey: ["environmentTheme"],
	queryFn: commands.environmentTheme,
});

export function ThemeSelector() {
	const queryClient = useQueryClient();
	const themeQuery = useQuery(environmentTheme);
	const changeTheme = useMutation({
		mutationFn: async (theme: string) =>
			await commands.environmentSetTheme(theme),
		onMutate: async (theme) => {
			document.documentElement.setAttribute("class", theme);
			await queryClient.invalidateQueries(environmentTheme);
			const data = queryClient.getQueryData(environmentTheme.queryKey);
			queryClient.setQueryData(environmentTheme.queryKey, theme);
			return data;
		},
		onError: (e, _, ctx) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentTheme.queryKey, ctx);
			if (ctx) document.documentElement.setAttribute("class", ctx);
		},
		onSettled: () => queryClient.invalidateQueries(environmentTheme),
	});

	return (
		<label className={"flex items-center"}>
			<span className={"text-lg"}>
				{tc("settings:theme")}
				{": "}
			</span>
			<Select value={themeQuery.data} onValueChange={changeTheme.mutate}>
				<SelectTrigger>
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						<SelectItem value={"system"}>
							{tc("settings:theme:system")}
						</SelectItem>
						<SelectItem value={"light"}>
							{tc("settings:theme:light")}
						</SelectItem>
						<SelectItem value={"dark"}>{tc("settings:theme:dark")}</SelectItem>
					</SelectGroup>
				</SelectContent>
			</Select>
		</label>
	);
}

const environmentGuiAnimation = queryOptions({
	queryKey: ["environmentGuiAnimation"],
	queryFn: commands.environmentGuiAnimation,
	initialData: true, // default value
});

export function GuiAnimationSwitch() {
	const queryClient = useQueryClient();
	const guiAnimation = useQuery(environmentGuiAnimation);
	const setShowPrerelease = useMutation({
		mutationFn: async (guiAnimation: boolean) =>
			await commands.environmentSetGuiAnimation(guiAnimation),
		onMutate: async (guiAnimation) => {
			await queryClient.cancelQueries(environmentGuiAnimation);
			const current = queryClient.getQueryData(
				environmentGuiAnimation.queryKey,
			);
			if (current != null) {
				queryClient.setQueryData(
					environmentGuiAnimation.queryKey,
					guiAnimation,
				);
			}
			return current;
		},
		onError: (e, _, prev) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentGuiAnimation.queryKey, prev);
		},
		onSuccess: (_, guiAnimation) => {
			document.dispatchEvent(
				new CustomEvent("gui-animation", { detail: guiAnimation }),
			);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGuiAnimation);
		},
	});

	return (
		<div>
			<label className={"flex items-center gap-2"}>
				<Checkbox
					checked={guiAnimation.data}
					onCheckedChange={(e) => setShowPrerelease.mutate(e === true)}
				/>
				{tc("settings:gui animation")}
			</label>
			<p className={"text-sm whitespace-normal"}>
				{tc("settings:gui animation description")}
			</p>
		</div>
	);
}

export function FilePathRow({
	path,
	notFoundMessage,
	pick,
	withOpen = true,
}: {
	path: string;
	notFoundMessage?: string;
	pick: () => void;
	withOpen?: boolean;
}) {
	const openFolder = async () => {
		try {
			await commands.utilOpen(path, "CreateFolderIfNotExists");
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	return (
		<div className={"flex gap-2 items-center"}>
			{!path && notFoundMessage ? (
				<Input
					className="flex-auto text-destructive"
					value={notFoundMessage}
					disabled
				/>
			) : (
				<Input className="flex-auto" value={path} disabled />
			)}
			<Button className={"flex-none px-4"} onClick={pick}>
				{tc("general:button:select")}
			</Button>
			{withOpen && (
				<Button className={"flex-none px-4"} onClick={openFolder}>
					{tc("general:button:open location")}
				</Button>
			)}
		</div>
	);
}

export function ProjectPathWarnings({ projectPath }: { projectPath: string }) {
	const globalInfo = useGlobalInfo();
	const isWindows = globalInfo.osType === "WindowsNT";
	const hasNonAscii = isWindows && projectPath.match(/[^\x20-\x7F]/);
	const hasWhitespace = projectPath.includes(" ");
	const inLocalAppData = !!(
		isWindows &&
		globalInfo.localAppData &&
		projectPath.includes(globalInfo.localAppData)
	);

	return (
		<div className="flex flex-col gap-1">
			{hasWhitespace && (
				<WarningMessage>{tc("settings:warning:whitespace")}</WarningMessage>
			)}
			{hasNonAscii && (
				<WarningMessage>{tc("settings:warning:non-ascii")}</WarningMessage>
			)}
			{inLocalAppData && (
				<WarningMessage>
					{tc("settings:warning:in-local-app-data")}
				</WarningMessage>
			)}
		</div>
	);
}

export function BackupPathWarnings({ backupPath }: { backupPath: string }) {
	const globalInfo = useGlobalInfo();
	const isWindows = globalInfo.osType === "WindowsNT";
	const inLocalAppData = !!(
		isWindows &&
		globalInfo.localAppData &&
		backupPath.includes(globalInfo.localAppData)
	);

	return (
		<div className="flex flex-col gap-1">
			{inLocalAppData && (
				<WarningMessage>
					{tc("settings:warning:in-local-app-data")}
				</WarningMessage>
			)}
		</div>
	);
}

export function WarningMessage({
	children,
}: {
	children: React.ReactNode;
}) {
	return (
		<div className={"flex items-center gap-2"}>
			<div className="grow-0 shrink-0">
				<CircleAlert className="text-warning w-5 h-5" />
			</div>
			<p className={"whitespace-normal text-sm"}>{children}</p>
		</div>
	);
}

export function BackupFormatSelect({
	backupFormat,
}: {
	backupFormat: string;
}) {
	const queryClient = useQueryClient();
	const setBackupFormat = useMutation({
		mutationFn: async (format: string) =>
			await commands.environmentSetBackupFormat(format),
		onMutate: async (format: string) => {
			await queryClient.cancelQueries(environmentGetSettings);
			const current = queryClient.getQueryData(environmentGetSettings.queryKey);
			if (current != null) {
				queryClient.setQueryData(environmentGetSettings.queryKey, {
					...current,
					backup_format: format,
				});
			}
			return current;
		},
		onError: (e, _, prev) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentGetSettings.queryKey, prev);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	return (
		<Select value={backupFormat} onValueChange={setBackupFormat.mutate}>
			<SelectTrigger>
				<SelectValue />
			</SelectTrigger>
			<SelectContent>
				<SelectGroup>
					<SelectItem value={"default"}>
						{tc("settings:backup:format:default")}
					</SelectItem>
					<SelectItem value={"zip-store"}>
						{tc("settings:backup:format:zip-store")}
					</SelectItem>
					<SelectItem value={"zip-fast"}>
						{tc("settings:backup:format:zip-fast")}
					</SelectItem>
					<SelectItem value={"zip-best"}>
						{tc("settings:backup:format:zip-best")}
					</SelectItem>
				</SelectGroup>
			</SelectContent>
		</Select>
	);
}
