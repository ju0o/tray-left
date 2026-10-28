param(
    [Parameter(Mandatory = $true)][string]$First,
    [Parameter(Mandatory = $true)][string]$Second,
    [Parameter(Mandatory = $true)][string]$OutputPath
)

$ErrorActionPreference = 'Stop'

function Read-U16([byte[]]$Bytes, [int]$Offset) { [BitConverter]::ToUInt16($Bytes, $Offset) }
function Read-U32([byte[]]$Bytes, [int]$Offset) { [BitConverter]::ToUInt32($Bytes, $Offset) }
function Hash-Range([byte[]]$Bytes, [int]$Offset, [int]$Length) {
    if ($Length -eq 0) { return '-' }
    $sha = [Security.Cryptography.SHA256]::Create()
    try { [BitConverter]::ToString($sha.ComputeHash($Bytes, $Offset, $Length)).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}
function Rva-Offset([uint32]$Rva, $Sections, [int]$HeadersSize) {
    foreach ($section in $Sections) {
        $span = [Math]::Max([int]$section.VirtualSize, [int]$section.RawSize)
        if ($Rva -ge $section.VirtualAddress -and $Rva -lt ($section.VirtualAddress + $span)) {
            $delta = [int]($Rva - $section.VirtualAddress)
            $offset = [int]$section.RawPointer + $delta
            if ($delta -ge 0 -and $delta -lt $section.RawSize -and $offset -lt $section.FileLength) { return $offset }
            throw 'A PE data directory points beyond the section data.'
        }
    }
    if ($Rva -lt $HeadersSize) { return [int]$Rva }
    throw 'A PE data directory RVA is outside every section.'
}

function Read-Pe([string]$Path, [string]$Label) {
    [byte[]]$bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 256 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) { throw "$Label is not a PE image." }
    $pe = [int](Read-U32 $bytes 0x3c)
    if ($pe -lt 64 -or $pe + 24 -gt $bytes.Length -or
        [Text.Encoding]::ASCII.GetString($bytes, $pe, 4) -ne "PE`0`0") { throw "$Label has an invalid PE header." }

    $coff = $pe + 4
    $machine = Read-U16 $bytes $coff
    $sectionCount = Read-U16 $bytes ($coff + 2)
    $timestamp = Read-U32 $bytes ($coff + 4)
    $symbolPointer = Read-U32 $bytes ($coff + 8)
    $symbolCount = Read-U32 $bytes ($coff + 12)
    $optionalSize = Read-U16 $bytes ($coff + 16)
    $optional = $coff + 20
    if ($optional + $optionalSize + 40 * $sectionCount -gt $bytes.Length) { throw "$Label has a truncated section table." }
    $magic = Read-U16 $bytes $optional
    if ($magic -eq 0x20b) { $directoryStart = $optional + 112; $numberOffset = $optional + 108 }
    elseif ($magic -eq 0x10b) { $directoryStart = $optional + 96; $numberOffset = $optional + 92 }
    else { throw "$Label uses an unsupported PE optional-header format." }
    $headersSize = [int](Read-U32 $bytes ($optional + 60))
    $imageSize = [int](Read-U32 $bytes ($optional + 56))
    $linkerVersion = '{0}.{1}' -f $bytes[$optional + 2], $bytes[$optional + 3]
    $directoryCount = [int](Read-U32 $bytes $numberOffset)
    $directories = @{}
    foreach ($index in @(2, 6)) {
        if ($index -lt $directoryCount) {
            $directories[$index] = @{
                Rva = [uint32](Read-U32 $bytes ($directoryStart + 8 * $index))
                Size = [int](Read-U32 $bytes ($directoryStart + 8 * $index + 4))
            }
        }
    }

    $sections = @()
    $sectionOffset = $optional + $optionalSize
    for ($index = 0; $index -lt $sectionCount; $index++) {
        $at = $sectionOffset + 40 * $index
        $name = [Text.Encoding]::ASCII.GetString($bytes, $at, 8).Trim([char]0)
        $virtualSize = [int](Read-U32 $bytes ($at + 8))
        $virtualAddress = [uint32](Read-U32 $bytes ($at + 12))
        $rawSize = [int](Read-U32 $bytes ($at + 16))
        $rawPointer = [int](Read-U32 $bytes ($at + 20))
        if ($rawSize -lt 0 -or $rawPointer -lt 0 -or $rawPointer + $rawSize -gt $bytes.Length) {
            throw "$Label section $name is outside the file."
        }
        $sections += [pscustomobject]@{
            Name = $name; VirtualSize = $virtualSize; VirtualAddress = $virtualAddress
            RawSize = $rawSize; RawPointer = $rawPointer; FileLength = $bytes.Length
            Hash = Hash-Range $bytes $rawPointer $rawSize
        }
    }
    if (-not ($sections.Name -contains '.rdata') -or -not ($sections.Name -contains '.rsrc')) {
        throw "$Label is missing the expected .rdata or .rsrc section."
    }

    $debugRows = @()
    if ($directories.ContainsKey(6) -and $directories[6].Rva -ne 0 -and $directories[6].Size -ge 28) {
        $debugOffset = Rva-Offset $directories[6].Rva $sections $headersSize
        $count = [Math]::Min(64, [int]($directories[6].Size / 28))
        for ($index = 0; $index -lt $count; $index++) {
            $at = $debugOffset + 28 * $index
            if ($at + 28 -gt $bytes.Length) { throw "$Label debug directory is truncated." }
            $type = Read-U32 $bytes ($at + 12)
            $size = [int](Read-U32 $bytes ($at + 16))
            $dataPointer = [int](Read-U32 $bytes ($at + 24))
            $stamp = Read-U32 $bytes ($at + 4)
            $codeView = 'no'
            $pdbLeaf = '-'
            $pdbPathHash = '-'
            if ($type -eq 2 -and $size -ge 25 -and $dataPointer -ge 0 -and $dataPointer + $size -le $bytes.Length -and
                [Text.Encoding]::ASCII.GetString($bytes, $dataPointer, 4) -eq 'RSDS') {
                $codeView = 'yes'
                $pathStart = $dataPointer + 24
                $pathEnd = $pathStart
                while ($pathEnd -lt ($dataPointer + $size) -and $bytes[$pathEnd] -ne 0) { $pathEnd++ }
                if ($pathEnd -gt $pathStart) {
                    $pdbPath = [Text.Encoding]::ASCII.GetString($bytes, $pathStart, $pathEnd - $pathStart)
                    $pdbLeaf = ($pdbPath -split '[\\/]')[-1]
                    $pdbPathHash = Hash-Range $bytes $pathStart ($pathEnd - $pathStart)
                }
            }
            $debugRows += "type=$type timestamp=$('{0:X8}' -f $stamp) size=$size rsds=$codeView pdb_leaf=$pdbLeaf pdb_path_sha256=$pdbPathHash"
        }
    }

    $rdata = $sections | Where-Object Name -eq '.rdata' | Select-Object -First 1
    $rdataSlice = New-Object byte[] $rdata.RawSize
    [Array]::Copy($bytes, $rdata.RawPointer, $rdataSlice, 0, $rdata.RawSize)
    $rdataBytes = [Text.Encoding]::ASCII.GetString($rdataSlice)
    $rdataUtf16 = [Text.Encoding]::Unicode.GetString($rdataSlice)
    $pathMarkers = @()
    foreach ($encoding in @(@{ Name = 'ascii'; Text = $rdataBytes }, @{ Name = 'utf16'; Text = $rdataUtf16 })) {
        foreach ($match in [regex]::Matches($encoding.Text, '(?i)[A-Z]:\\[^\x00\r\n"<>|?*]{1,260}')) {
            $value = $match.Value.TrimEnd([char]0)
            $leaf = ($value -split '[\\/]')[-1]
            if ($leaf) {
                $pathBytes = [Text.Encoding]::UTF8.GetBytes($value)
                $pathMarkers += "encoding=$($encoding.Name) leaf=$leaf length=$($value.Length) sha256=$([Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($pathBytes)).ToLowerInvariant())"
            }
        }
    }

    return [pscustomobject]@{
        Label = $Label; Machine = ('0x{0:X4}' -f $machine); Timestamp = ('0x{0:X8}' -f $timestamp)
        OptionalMagic = ('0x{0:X4}' -f $magic); LinkerVersion = $linkerVersion
        ImageSize = $imageSize; HeadersSize = $headersSize; CoffSymbolPointer = $symbolPointer
        CoffSymbolCount = $symbolCount; Sections = $sections; Directories = $directories
        Debug = $debugRows; PathMarkers = $pathMarkers
    }
}

$a = Read-Pe $First 'A'
$b = Read-Pe $Second 'B'
$lines = [Collections.Generic.List[string]]::new()
foreach ($pe in @($a, $b)) {
    $lines.Add("[$($pe.Label)] machine=$($pe.Machine) coff_timestamp=$($pe.Timestamp) optional_magic=$($pe.OptionalMagic) linker_version=$($pe.LinkerVersion) image_size=$($pe.ImageSize) headers_size=$($pe.HeadersSize) coff_symbol_pointer=$($pe.CoffSymbolPointer) coff_symbol_count=$($pe.CoffSymbolCount)")
    foreach ($index in @(2, 6)) {
        if ($pe.Directories.ContainsKey($index)) {
            $name = if ($index -eq 2) { 'resource' } else { 'debug' }
            $entry = $pe.Directories[$index]
            $lines.Add("  directory_$name rva=$('{0:X8}' -f $entry.Rva) size=$($entry.Size)")
        }
    }
    foreach ($section in $pe.Sections) {
        $lines.Add("  section=$($section.Name) va=$('{0:X8}' -f $section.VirtualAddress) virtual=$($section.VirtualSize) raw_pointer=$('{0:X8}' -f $section.RawPointer) raw_size=$($section.RawSize) sha256=$($section.Hash)")
    }
    foreach ($row in $pe.Debug) { $lines.Add("  debug $row") }
    foreach ($row in $pe.PathMarkers) { $lines.Add("  rdata_path_marker $row") }
}
foreach ($name in @('.text', '.rdata', '.rsrc', '.pdata', '.reloc')) {
    $sectionA = $a.Sections | Where-Object Name -eq $name | Select-Object -First 1
    $sectionB = $b.Sections | Where-Object Name -eq $name | Select-Object -First 1
    if ($sectionA -or $sectionB) {
        $same = $sectionA -and $sectionB -and $sectionA.RawSize -eq $sectionB.RawSize -and $sectionA.Hash -eq $sectionB.Hash
        $lines.Add("compare_section=$name identical=$([bool]$same)")
    }
}
$lines.Add("compare_coff_timestamp_identical=$($a.Timestamp -eq $b.Timestamp)")
$lines.Add("compare_debug_directory_identical=$(($a.Debug -join ';') -eq ($b.Debug -join ';'))")
[IO.File]::WriteAllLines($OutputPath, $lines, [Text.UTF8Encoding]::new($false))
Get-Content -LiteralPath $OutputPath
