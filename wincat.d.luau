type Process = {
	name: string,
	pid: number,
	main: Window?,
	windows: {[number]: Window}
}

type Window = {
	title: string,
	class_name: string,
	hwnd: number,
	visible: boolean,
	x: number,
	y: number,
	width: number,
	height: number
}

type ProcessList = {[number]: Process}
type Callback = (procs: ProcessList) -> number?
