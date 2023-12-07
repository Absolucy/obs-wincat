return function(procs)
	for i, proc in ipairs(procs) do
		print("Process " .. tostring(i) .. " has title " .. tostring(proc.name) .. " and pid " .. tostring(proc.pid))
	end
end
