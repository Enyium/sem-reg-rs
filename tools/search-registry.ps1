function Search-RegistryBinaryData {
    param(
        [string[]] $Paths,
        [scriptblock] $Verify
    )

    function Write-WindowPadded {
        param(
            [string] $Text,
            [switch] $NoNewLine
        )

        $maxWidth = [Console]::WindowWidth - 1
        $paddedText = $Text.Substring(0, [Math]::Min($Text.Length, $maxWidth)).PadRight($maxWidth)
        Write-Host $paddedText -NoNewline:$NoNewLine
    }

    try {
        $occurrences = 0
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

        foreach ($userPath in $Paths) {
            Get-ChildItem -LiteralPath "Registry::$userPath" -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
                $keyPath = $_
                $key = Get-Item -LiteralPath "Registry::$keyPath"

                foreach ($valueName in $key.GetValueNames()) {
                    if ($stopwatch.ElapsedMilliseconds -ge 500) {
                        Write-WindowPadded "Searching in: $keyPath" -NoNewline
                        Write-Host "`r" -NoNewline
                        $stopwatch.Restart()
                    }

                    if ($key.GetValueKind($valueName) -eq [Microsoft.Win32.RegistryValueKind]::Binary) {
                        $hexString = ($key.GetValue($valueName) | ForEach-Object { $_.ToString("x2") }) -join ' '
                        if (& $Verify $hexString) {
                            Write-Host "$keyPath\$valueName"
                            $occurrences++
                        }
                    }
                }
            }
        }
    } finally {  # Also on Ctrl+C.
        Write-WindowPadded
        Write-Host "Found $occurrences binary values."
    }
}

function Search-RegistryBinaryDataWithRegex {
    param(
        [string[]] $Paths,
        [string] $RegexString
    )

    $regex = [System.Text.RegularExpressions.Regex]::new($RegexString)
    Search-RegistryBinaryData -Paths $Paths -Verify {
        param($hexString)
        $regex.IsMatch($hexString)
    }
}

################################################################################

# Search-RegistryBinaryDataWithRegex -Paths @('HKCU') -RegexString '(?i)^43 42 01 00 0a 02 01 00 (?!2a 06)'
# Search-RegistryBinaryDataWithRegex -Paths @('HKCU') -RegexString '(?i)2a 2b 0e .. (?!43 42 01)'
# Search-RegistryBinaryDataWithRegex -Paths @('HKCU') -RegexString '(?i)^43 42 01 00((?!43 42 01 00).)*$'
# Search-RegistryBinaryDataWithRegex -Paths @('HKCU') -RegexString '(?i)^(?!43 42 01 00 0a 00)43 42 01 00((?!43 42 01 00).)*$'
Search-RegistryBinaryDataWithRegex -Paths @('HKCU') -RegexString '(?i)^.{1703}$'
