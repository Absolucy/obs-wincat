return function(procs)
	for i, proc in ipairs(procs) do
		if proc.name == "dreamseeker.exe" and type(proc.main) == "table" then
			return proc.main
		end
	end
	return nil
end
