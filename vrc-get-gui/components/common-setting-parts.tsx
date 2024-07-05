import i18next, {languages, tc} from "@/lib/i18n";
import {Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue} from "@/components/ui/select";
import React from "react";
import {useQuery} from "@tanstack/react-query";
import {environmentLanguage, environmentSetLanguage, environmentSetTheme, environmentTheme} from "@/lib/bindings";

export function LanguageSelector() {
	const {data: lang, refetch: refetchLang} = useQuery({
		queryKey: ["environmentLanguage"],
		queryFn: environmentLanguage
	})

	const changeLanguage = async (value: string) => {
		await Promise.all([
			i18next.changeLanguage(value),
			environmentSetLanguage(value),
			refetchLang(),
		])
	};

	return (
		<label className="flex items-center">
			<span className="text-lg">
				{tc("settings:language")}{": "}
			</span>
			<Select value={lang} onValueChange={changeLanguage}>
				<SelectTrigger>
					<SelectValue/>
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						{
							languages.map((lang) => (
								<SelectItem key={lang} value={lang}>{tc("settings:langName", {lng: lang})}</SelectItem>
							))
						}
					</SelectGroup>
				</SelectContent>
			</Select>
		</label>
	)
}

export function ThemeSelector() {
	const [theme, setTheme] = React.useState<string | null>(null);

	React.useEffect(() => {
		(async () => {
			const theme = await environmentTheme();
			setTheme(theme);
		})();
	}, [])

	const changeTheme = async (theme: string) => {
		await environmentSetTheme(theme);
		setTheme(theme);
		if (theme === "system") {
			const {appWindow} = await import("@tauri-apps/api/window");
			theme = await appWindow.theme() ?? "light";
		}
		document.documentElement.setAttribute("class", theme);
	};

	return (
		<label className={"flex items-center"}>
			<span className={"text-lg"}>{tc("settings:theme")}{": "}</span>
			<Select value={theme ?? undefined} onValueChange={changeTheme}>
				<SelectTrigger>
					<SelectValue/>
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						<SelectItem value={"system"}>{tc("settings:theme:system")}</SelectItem>
						<SelectItem value={"light"}>{tc("settings:theme:light")}</SelectItem>
						<SelectItem value={"dark"}>{tc("settings:theme:dark")}</SelectItem>
					</SelectGroup>
				</SelectContent>
			</Select>
		</label>
	)
}
