DECLARE SUB FileLoad ()
DECLARE SUB PlatBegin ()
DECLARE SUB FileSave ()
DEFINT A-Z
DECLARE SUB FileSelect ()
DECLARE FUNCTION GetFileCount% (FileSpec$)
DECLARE SUB ColorDisplay ()
DECLARE SUB SetupMenu ()
DECLARE SUB SetupDesktop ()
DECLARE SUB ShowResults ()
DECLARE FUNCTION BearDec# (dms#)
DECLARE FUNCTION FmtBear$ (dec#)

'$INCLUDE: 'general.bi'
'$INCLUDE: 'mouse.bi'
'$INCLUDE: 'menu.bi'
'$INCLUDE: 'window.bi'

COMMON SHARED /uitools/ GloMenu           AS MenuMiscType
COMMON SHARED /uitools/ GloTitle()        AS MenuTitleType
COMMON SHARED /uitools/ GloItem()         AS MenuItemType
COMMON SHARED /uitools/ GloWindow()       AS windowType
COMMON SHARED /uitools/ GloButton()       AS buttonType
COMMON SHARED /uitools/ GloEdit()         AS EditFieldType
COMMON SHARED /uitools/ GloStorage        AS WindowStorageType
COMMON SHARED /uitools/ GloWindowStack()  AS INTEGER
COMMON SHARED /uitools/ GloBuffer$()

DIM GloTitle(MAXMENU)           AS MenuTitleType
DIM GloItem(MAXMENU, MAXITEM)   AS MenuItemType
DIM GloWindow(MAXWINDOW)        AS windowType
DIM GloButton(MAXBUTTON)        AS buttonType
DIM GloEdit(MAXEDITFIELD)       AS EditFieldType
DIM GloWindowStack(MAXWINDOW)   AS INTEGER
DIM GloBuffer$(MAXWINDOW + 1, 2)

TYPE CourseType
    Quad     AS STRING * 1
    Bearing  AS DOUBLE
    Hemi     AS STRING * 1
    Distance AS DOUBLE
    DeltaN   AS DOUBLE
    DeltaE   AS DOUBLE
END TYPE

CONST MAXCOURSES = 200

DIM SHARED Courses(1 TO MAXCOURSES) AS CourseType
DIM SHARED NumCourses               AS INTEGER
DIM SHARED PlatName                 AS STRING
DIM SHARED MapFileName              AS STRING

DIM SHARED DisplayType  AS INTEGER
BeginFlag1 = TRUE

Begin:

	MenuInit
	WindowInit
	MouseShow
	ColorDisplay

IF BeginFlag1 = TRUE THEN GOSUB Credits


MapCheckFinished = FALSE

WHILE NOT MapCheckFinished
	kbd$ = MenuInkey$
	WHILE MenuCheck(2)
	    GOSUB MenuTrap
	WEND
    WEND

	MouseHide
	COLOR 15, 0
	CLS
	END

MenuTrap:
    menu = MenuCheck(0)
    item = MenuCheck(1)

    SELECT CASE menu
	CASE 1
		SELECT CASE item
			CASE 1: FileSelect
			CASE 2: FileSave
			CASE 3: FileLoad
			CASE 4: CLS : SHELL: CLS : GOTO Begin
			CASE 5: MapCheckFinished = TRUE
			END SELECT

	  CASE 2
		SELECT CASE item
			CASE 1: PlatBegin
		END SELECT

	  CASE ELSE
		END SELECT

RETURN

Credits:
	a$ = "Map Checker |"
	a$ = a$ + "By |"
	a$ = a$ + "Stu Cameron |"
	a$ = a$ + "Copyright (c) 1992 CamWare"
	x = Alert(4, a$, 9, 10, 14, 70, "", "", "")
	BeginFlag1 = FALSE
RETURN




SUB ColorDisplay

    MouseHide
    SetupMenu
    SetupDesktop
    MenuShow
    MouseShow
END SUB

' -----------------------------------------------------------------------
' Convert DD.MMSS to decimal degrees
' -----------------------------------------------------------------------
FUNCTION BearDec# (dms#)
    sign# = 1
    IF dms# < 0 THEN sign# = -1: dms# = ABS(dms#)
    deg# = INT(dms#)
    frac# = (dms# - deg#) * 100
    mm# = INT(frac# + .0001)
    ss# = (frac# - mm#) * 100
    BearDec# = sign# * (deg# + mm# / 60# + ss# / 3600#)
END FUNCTION

' -----------------------------------------------------------------------
' Format decimal degrees as DD-MM-SS string
' -----------------------------------------------------------------------
FUNCTION FmtBear$ (dec#)
    IF dec# = 0 THEN FmtBear$ = "00-00-00": EXIT FUNCTION
    d% = INT(dec#)
    m% = INT((dec# - d%) * 60)
    s# = ((dec# - d%) * 60 - m%) * 60
    FmtBear$ = LTRIM$(STR$(d%)) + "-" + RIGHT$("0" + LTRIM$(STR$(m%)), 2) + "-" + RIGHT$("0" + LTRIM$(STR$(INT(s# + .5))), 2)
END FUNCTION

' -----------------------------------------------------------------------
' FileLoad - load a saved .MAP file
' -----------------------------------------------------------------------
SUB FileLoad

    WindowOpen 1, 8, 20, 13, 60, 0, 7, 0, 7, 15, FALSE, FALSE, FALSE, TRUE, 1, ""
    WindowLocate 2, 4
    WindowPrint 4, "Enter filename to load:"
    WindowBox 3, 4, 5, 38
    EditFieldOpen 1, "*.MAP", 4, 5, 0, 0, 23, 22
    WindowLine 7
    ButtonOpen 1, 2, "OK", 8, 17, 0, 0, 1

    currButton = 0
    currEditField = 1
    ExitFlag = FALSE

    WHILE NOT ExitFlag
        WindowDo currButton, currEditField
        SELECT CASE Dialog(0)
            CASE 1, 6
                ExitFlag = TRUE
            CASE 7, 8
                IF currButton = 1 THEN
                    currButton = 0: currEditField = 1
                ELSE
                    currButton = 1: currEditField = 0
                END IF
            CASE 9
                WindowClose 1
                EXIT SUB
        END SELECT
    WEND

    LoadName$ = RTRIM$(EditFieldInquire$(1))
    WindowClose 1

    IF LEN(LoadName$) = 0 THEN EXIT SUB
    IF INSTR(LoadName$, ".") = 0 THEN LoadName$ = LoadName$ + ".MAP"

    IF DIR$(LoadName$) = "" THEN
        junk = Alert(4, "File not found: " + LoadName$, 10, 10, 12, 70, "OK", "", "")
        EXIT SUB
    END IF

    OPEN LoadName$ FOR INPUT AS #1
    LINE INPUT #1, PlatName
    INPUT #1, NumCourses
    FOR i% = 1 TO NumCourses
        LINE INPUT #1, Courses(i%).Quad
        INPUT #1, Courses(i%).Bearing
        LINE INPUT #1, Courses(i%).Hemi
        INPUT #1, Courses(i%).Distance
        rad# = Courses(i%).Bearing * .01745329252#
        dn# = Courses(i%).Distance * COS(rad#)
        de# = Courses(i%).Distance * SIN(rad#)
        IF Courses(i%).Quad = "N" THEN Courses(i%).DeltaN = dn# ELSE Courses(i%).DeltaN = -dn#
        IF Courses(i%).Hemi = "E" THEN Courses(i%).DeltaE = de# ELSE Courses(i%).DeltaE = -de#
    NEXT i%
    CLOSE #1

    MapFileName = LoadName$
    CALL ShowResults

END SUB

' -----------------------------------------------------------------------
' FileSave - save current plat to a .MAP file
' -----------------------------------------------------------------------
SUB FileSave

    IF NumCourses = 0 THEN
        junk = Alert(4, "No plat data to save.", 10, 15, 12, 62, "OK", "", "")
        EXIT SUB
    END IF

    WindowOpen 1, 8, 20, 13, 60, 0, 7, 0, 7, 15, FALSE, FALSE, FALSE, TRUE, 1, ""
    WindowLocate 2, 4
    WindowPrint 4, "Save file as:"
    WindowBox 3, 4, 5, 38
    EditFieldOpen 1, MapFileName, 4, 5, 0, 0, 23, 22
    WindowLine 7
    ButtonOpen 1, 2, "OK", 8, 17, 0, 0, 1

    currButton = 0
    currEditField = 1
    ExitFlag = FALSE

    WHILE NOT ExitFlag
        WindowDo currButton, currEditField
        SELECT CASE Dialog(0)
            CASE 1, 6
                ExitFlag = TRUE
            CASE 7, 8
                IF currButton = 1 THEN
                    currButton = 0: currEditField = 1
                ELSE
                    currButton = 1: currEditField = 0
                END IF
            CASE 9
                WindowClose 1
                EXIT SUB
        END SELECT
    WEND

    SaveName$ = RTRIM$(EditFieldInquire$(1))
    WindowClose 1

    IF LEN(SaveName$) = 0 THEN EXIT SUB
    IF INSTR(SaveName$, ".") = 0 THEN SaveName$ = SaveName$ + ".MAP"

    OPEN SaveName$ FOR OUTPUT AS #1
    PRINT #1, RTRIM$(PlatName)
    PRINT #1, NumCourses
    FOR i% = 1 TO NumCourses
        PRINT #1, Courses(i%).Quad
        PRINT #1, Courses(i%).Bearing
        PRINT #1, Courses(i%).Hemi
        PRINT #1, Courses(i%).Distance
    NEXT i%
    CLOSE #1

    MapFileName = SaveName$
    junk = Alert(4, "Saved: " + SaveName$, 10, 15, 12, 62, "OK", "", "")

END SUB

' -----------------------------------------------------------------------
' FileSelect - browse and open a .MAP file
' -----------------------------------------------------------------------
SUB FileSelect

    WindowOpen 1, 8, 20, 15, 60, 0, 7, 0, 7, 15, FALSE, FALSE, FALSE, TRUE, 1, ""

    WindowLocate 2, 4
    WindowPrint 4, "Enter a file specification:"
    WindowBox 3, 4, 5, 38

    EditFieldOpen 1, "*.MAP", 4, 5, 0, 0, 23, 22
    WindowLine 7
    ButtonOpen 1, 2, "OK", 8, 17, 0, 0, 1

    currButton = 0
    currEditField = 1

    ExitFlag = FALSE
    WHILE NOT ExitFlag
	   WindowDo currButton, currEditField
	   SELECT CASE Dialog(0)
		  CASE 1, 6
			 ExitFlag = TRUE
		  CASE 2
			 currButton = 0
			 currEditField = 1
		  CASE 7, 8
			 IF currButton = 1 THEN
				currButton = 0
				currEditField = 1
			 ELSE
				currButton = 1
				currEditField = 0
			 END IF
		  CASE 9
			 WindowClose 1
			 EXIT SUB
		  CASE 14
			 IF currButton = 1 THEN
				ExitFlag = TRUE
			 END IF
		  CASE ELSE
	   END SELECT
    WEND

    FileSpec$ = EditFieldInquire$(1)

    delimit = INSTR(FileSpec$, ".")

    IF delimit THEN
	   fileName$ = LEFT$(FileSpec$, delimit - 1)
	   fileExt$ = RIGHT$(FileSpec$, LEN(FileSpec$) - (delimit))
    ELSE
	   fileName$ = FileSpec$
	   fileExt$ = ""
    END IF

    IF LEN(FileSpec$) = 0 OR LEN(fileName$) > 8 OR LEN(fileExt$) > 3 THEN
	   WindowClose 1
	   junk = Alert(4, "You didn't enter a valid file specification.", 10, 15, 12, 62, "OK", "", "")
	   EXIT SUB
    END IF

    FileCount = GetFileCount(FileSpec$)

    IF FileCount THEN

	   REDIM FileList$(FileCount)

    ELSE

	   WindowClose 1
	   junk = Alert(4, "No match to your file specification could be found.", 10, 10, 12, 70, "OK", "", "")
	   EXIT SUB
    END IF

    FileList$(1) = DIR$(FileSpec$)

    FOR Indx = 2 TO FileCount
	   FileList$(Indx) = DIR$
    NEXT Indx

    x = ListBox(FileList$(), UBOUND(FileList$))

    SELECT CASE x
	   CASE 0
		  junk = Alert(4, "You selected CANCEL", 10, 25, 12, 55, "OK", "", "")
	   CASE ELSE
		  WindowClose 1
		  CALL _LoadFile(FileList$(x))
		  EXIT SUB
    END SELECT

    WindowClose 1

END SUB

' -----------------------------------------------------------------------
' Internal helper - load a specific file by name
' -----------------------------------------------------------------------
SUB _LoadFile (fname$)

    IF DIR$(fname$) = "" THEN
        junk = Alert(4, "File not found: " + fname$, 10, 10, 12, 70, "OK", "", "")
        EXIT SUB
    END IF

    OPEN fname$ FOR INPUT AS #1
    LINE INPUT #1, PlatName
    INPUT #1, NumCourses
    FOR i% = 1 TO NumCourses
        LINE INPUT #1, Courses(i%).Quad
        INPUT #1, Courses(i%).Bearing
        LINE INPUT #1, Courses(i%).Hemi
        INPUT #1, Courses(i%).Distance
        rad# = Courses(i%).Bearing * .01745329252#
        dn# = Courses(i%).Distance * COS(rad#)
        de# = Courses(i%).Distance * SIN(rad#)
        IF Courses(i%).Quad = "N" THEN Courses(i%).DeltaN = dn# ELSE Courses(i%).DeltaN = -dn#
        IF Courses(i%).Hemi = "E" THEN Courses(i%).DeltaE = de# ELSE Courses(i%).DeltaE = -de#
    NEXT i%
    CLOSE #1
    MapFileName = fname$
    CALL ShowResults

END SUB

' -----------------------------------------------------------------------
' GetFileCount - count files matching a spec
' -----------------------------------------------------------------------
FUNCTION GetFileCount% (FileSpec$)
    cnt% = 0
    f$ = DIR$(FileSpec$)
    WHILE f$ <> ""
        cnt% = cnt% + 1
        f$ = DIR$
    WEND
    GetFileCount% = cnt%
END FUNCTION

' -----------------------------------------------------------------------
' PlatBegin - enter courses and compute map check closure
' -----------------------------------------------------------------------
SUB PlatBegin

    ' --- Get plat name ---
    WindowOpen 1, 8, 20, 12, 60, 0, 7, 0, 7, 15, FALSE, FALSE, FALSE, TRUE, 1, ""
    WindowLocate 2, 4
    WindowPrint 4, "Enter plat name:"
    WindowBox 3, 4, 5, 38
    EditFieldOpen 1, "", 4, 5, 0, 0, 23, 30
    WindowLine 7
    ButtonOpen 1, 2, "OK", 8, 17, 0, 0, 1

    currButton = 0: currEditField = 1: ExitFlag = FALSE
    WHILE NOT ExitFlag
        WindowDo currButton, currEditField
        SELECT CASE Dialog(0)
            CASE 1, 6: ExitFlag = TRUE
            CASE 7, 8
                IF currButton = 1 THEN
                    currButton = 0: currEditField = 1
                ELSE
                    currButton = 1: currEditField = 0
                END IF
            CASE 9: WindowClose 1: EXIT SUB
        END SELECT
    WEND
    PlatName = RTRIM$(EditFieldInquire$(1))
    WindowClose 1

    IF LEN(RTRIM$(PlatName)) = 0 THEN EXIT SUB

    ' --- Enter courses ---
    NumCourses = 0
    MouseHide
    VIEW PRINT
    CLS
    COLOR 14: PRINT "MAP CHECK - "; RTRIM$(PlatName): COLOR 15
    PRINT "Enter courses.  Bearing in DD.MMSS format (eg. 45.3015 = 45-30-15)"
    PRINT "Enter Q or q at Quadrant prompt to finish."
    PRINT STRING$(79, "-")

platloop:

    IF NumCourses >= MAXCOURSES THEN
        PRINT "Maximum courses reached ("; MAXCOURSES; ")."
        GOTO platdone
    END IF

    n% = NumCourses + 1
    PRINT "Course "; n%; ":"

    ' Quadrant
    INPUT "  Quadrant (N/S, Q to finish): ", q$
    q$ = UCASE$(LEFT$(q$, 1))
    IF q$ = "Q" THEN GOTO platdone
    IF q$ <> "N" AND q$ <> "S" THEN
        PRINT "  Must be N or S."
        GOTO platloop
    END IF

    ' Bearing angle
    INPUT "  Bearing (DD.MMSS): ", b#
    bd# = BearDec#(b#)
    IF bd# < 0 OR bd# > 90 THEN
        PRINT "  Bearing must be between 0 and 90 degrees."
        GOTO platloop
    END IF

    ' Hemisphere
    INPUT "  Hemisphere (E/W): ", h$
    h$ = UCASE$(LEFT$(h$, 1))
    IF h$ <> "E" AND h$ <> "W" THEN
        PRINT "  Must be E or W."
        GOTO platloop
    END IF

    ' Distance
    INPUT "  Distance: ", d#
    IF d# <= 0 THEN
        PRINT "  Distance must be greater than zero."
        GOTO platloop
    END IF

    ' Store course and compute deltas
    NumCourses = n%
    Courses(n%).Quad = q$
    Courses(n%).Bearing = bd#
    Courses(n%).Hemi = h$
    Courses(n%).Distance = d#
    rad# = bd# * .01745329252#
    dn# = d# * COS(rad#)
    de# = d# * SIN(rad#)
    IF q$ = "N" THEN Courses(n%).DeltaN = dn# ELSE Courses(n%).DeltaN = -dn#
    IF h$ = "E" THEN Courses(n%).DeltaE = de# ELSE Courses(n%).DeltaE = -de#

    bear$ = q$ + " " + FmtBear$(bd#) + " " + h$
    PRINT "  Added: "; bear$; "  "; : PRINT USING "########.###"; d#
    PRINT

    GOTO platloop

platdone:

    IF NumCourses = 0 THEN
        PRINT "No courses entered."
        COLOR 5: PRINT "Press any key...": COLOR 15
        DO: LOOP WHILE INKEY$ = ""
        MouseShow
        EXIT SUB
    END IF

    MapFileName = ""
    MouseShow
    CALL ShowResults

END SUB

' -----------------------------------------------------------------------
' ShowResults - display closure report for current plat
' -----------------------------------------------------------------------
SUB ShowResults

    MouseHide
    VIEW PRINT
    CLS
    COLOR 14
    PRINT "MAP CHECK RESULTS - "; RTRIM$(PlatName)
    COLOR 15
    PRINT STRING$(79, "=")
    PRINT USING "###  "; 0;
    PRINT "Bearing                  ";
    PRINT USING "############"; 0;
    PRINT "     Delta N         Delta E"
    PRINT USING "###  "; 0;
    PRINT "-------                  ";
    PRINT USING "------------"; 0;
    PRINT "     --------         -------"

    ' Print header properly
    LOCATE CSRLIN - 2, 1
    PRINT "  #  Bearing                    Distance       Delta N       Delta E"
    PRINT STRING$(79, "-")

    totalDist# = 0#
    totalN# = 0#
    totalE# = 0#

    FOR i% = 1 TO NumCourses
        bear$ = Courses(i%).Quad + " " + FmtBear$(Courses(i%).Bearing) + " " + Courses(i%).Hemi
        PRINT USING "###  "; i%;
        PRINT bear$; SPACE$(20 - LEN(bear$));
        PRINT USING "###########.###"; Courses(i%).Distance;
        PRINT USING "  ###########.####"; Courses(i%).DeltaN;
        PRINT USING "  ###########.####"; Courses(i%).DeltaE
        totalDist# = totalDist# + Courses(i%).Distance
        totalN# = totalN# + Courses(i%).DeltaN
        totalE# = totalE# + Courses(i%).DeltaE

        ' Pause every 18 lines
        IF (CSRLIN >= 22) AND (i% < NumCourses) THEN
            COLOR 5: PRINT "Press any key for more...": COLOR 15
            DO: LOOP WHILE INKEY$ = ""
            CLS
            PRINT "  #  Bearing                    Distance       Delta N       Delta E"
            PRINT STRING$(79, "-")
        END IF
    NEXT i%

    PRINT STRING$(79, "=")
    PRINT "     TOTALS";
    PRINT SPACE$(15);
    PRINT USING "###########.###"; totalDist#;
    PRINT USING "  ###########.####"; totalN#;
    PRINT USING "  ###########.####"; totalE#
    PRINT

    error# = SQR(totalN# ^ 2 + totalE# ^ 2)
    PRINT USING "  Error of Closure : ###########.####"; error#
    IF error# > 0 THEN
        prec# = totalDist# / error#
        PRINT USING "  Precision        : 1 : ##########"; prec#
    ELSE
        PRINT "  Precision        : PERFECT CLOSURE"
    END IF
    PRINT USING "  Total Distance   : ###########.###"; totalDist#

    PRINT
    COLOR 5: PRINT "Press any key to continue...": COLOR 15
    DO: LOOP WHILE INKEY$ = ""
    MouseShow

END SUB

DEFSNG A-Z
SUB SetupDesktop STATIC

    MouseHide

    WIDTH , 25

	    COLOR 15, 1      'Color

    CLS


    MouseShow
END SUB

DEFINT A-Z
SUB SetupMenu

	MenuSet 1, 0, 1, "Files", 1
	MenuSet 1, 1, 1, "Select Files         F1", 1
	MenuSet 1, 2, 1, "Save Current File    F2", 2
	MenuSet 1, 3, 1, "Load a Saved File    F3", 1
	MenuSet 1, 4, 1, "Dos Shell            F4", 1
	MenuSet 1, 5, 1, "Exit                 Ctrl-X", 2

	MenuSet 2, 0, 1, "Enter A Plat", 1
	MenuSet 2, 1, 1, "Begin New Plat       Ctrl-B", 1

    ShortCutKeySet 1, 1, CHR$(0) + CHR$(59) ' F1
    ShortCutKeySet 1, 2, CHR$(0) + CHR$(60) ' F2
    ShortCutKeySet 1, 3, CHR$(0) + CHR$(61) ' F3
    ShortCutKeySet 1, 4, CHR$(0) + CHR$(62) ' F4
    ShortCutKeySet 1, 5, CHR$(24)           ' Ctrl-X
    ShortCutKeySet 2, 1, CHR$(2)

    MenuColor 0, 7, 4, 8, 15, 0, 12
    MenuPreProcess

END SUB
