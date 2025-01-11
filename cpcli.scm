#!/usr/bin/env guile
!#

(use-modules 
  (rnrs bytevectors)
  (srfi srfi-1)
  (srfi srfi-9)
  (web client)
  (web response)
  (json))

(define help "
Options:
    [TOKEN]              Current value of TOKEN in fiat
    [TOKEN:N]            Convert N of fiat to TOKEN
    [N:TOKEN]            Convert N of TOKEN to fiat
    -c C | --currency C  Use fiat currency C (default is usd)
    -h | --help          Display this help message
    -t N | --top N       Top N tokens by market cap

")

(define api-fail-msg "Malformed API response, usually caused by misspelled currency or too many requests")

(define-syntax-rule (value-assoc find kvp)
  (cdr (assoc find kvp)))

(define-syntax-rule (push l i) 
  (set! l (append l (list i))))

(define-record-type token
  (make-token name value)
  token?
  (name token-name)
  (value token-value))

(define-record-type conv
  (make-conv name amount ftoc)
  conv?
  (name conv-name)
  (amount conv-amount)
  (ftoc conv-ftoc))

(define (api-call endpoint)
  (catch #t
    (lambda () (json-string->scm (utf8->string (read-response-body 
      (http-get 
        endpoint 
        #:streaming? #t
        #:headers '((user-agent . "cpcli")))))))
    (lambda e (err (format #f "API call failed: ~a~%Endpoint: ~a" e endpoint)))))

(define (err msg) 
  (format (current-error-port) "~a~%" msg) 
  (exit 1))

(define (token-index lst name)
  (fold
    (lambda (i acc)
      (if acc
        acc
        (catch #t
          (lambda () (if (equal? (car (list-ref lst i)) name) i #f))
          (lambda e #f))))
    #f
    (iota (length lst) 0)))

(define MAGNITUDE (list " " "K" "M" "B" "T"))

; Configurable global variables:
(define CONV-DIGITS 5)
(define RANK-LEN 3)
(define NAME-LEN 8)
(define VALUE-LEN 7)
(define CHANGE-LEN 7)
(define CONVS-PADDING (format #f "~3_"))
(define TOKENS-PADDING (format #f "~5_"))
(define TOP-PADDING (format #f "~3_"))

(define (format-rank n)
  (let ((s (number->string n)))
    (string-pad-right s RANK-LEN)))

(define (format-name name capitalized)
  (let
      ((len (string-length name))
      (name (if capitalized name (string-titlecase name))))
    (if (> len NAME-LEN)
      (string-append (string-take name (- NAME-LEN 1)) "-")
      (string-pad-right name NAME-LEN))))

(define (format-value value)
  (when (< value 0)
    (err (format #f "Invalid value: ~a" value)))
  (if (<= value 1)
    (format #f
      (string-append
        "~,"
        (number->string (- VALUE-LEN 2))
        "h")
      value)
    (let ((mag (max (inexact->exact (floor (/ (log10 value) 3))) 0)))
      (if (> mag (length MAGNITUDE))
        (string-pad-right "PUMPED!" VALUE-LEN))
        (let* ((scale (- (* mag 3))))
          (string-pad-right
            (format #f
              (string-append
                "~,2,"
                (number->string (inexact->exact scale))
                "f"
                (list-ref MAGNITUDE mag))
              value)
            VALUE-LEN)))))

(define (format-change change)
  (let*
      ((l10 (max (inexact->exact (floor (log10 (abs change)))) 0))
      (decimals (- CHANGE-LEN l10 4))) ; - 4 for sign, decimal, %, and 1s place
    (when (>= l10 (- CHANGE-LEN 1)) 
      (string-pad-right "PUMPED!" CHANGE-LEN))
    (format #f
      (string-append
        (if (> change 0) "+" "")
        "~," 
        (number->string decimals)
        "h%")
      change)))

(define args (cdr (program-arguments)))
(define fiat (cons "usd" #f))

(define convs '())
(define conv-names '())
(define tokens '())
(define top (cons 0 #f))

(when (equal? (length args) 0)
  (display help) (exit 1))
(map
  (lambda (arg)
    (let ((arg-check (or (cdr fiat) (cdr top))))
      (cond 
        ((or (string=? arg "-h") (string=? arg "--help"))
          (display help) (exit 0))
        ((or (string=? arg "-c") (string=? arg "--currency"))
          (if arg-check 
              (err "Missing value for -c or -t")
              (set-cdr! fiat #t)))
        ((or (string=? arg "-t") (string=? arg "--top"))
          (if arg-check 
            (err "Missing value for -c or -t")
            (set-cdr! top #t)))
        (else 
          (cond 
            ((cdr fiat) 
              (set! fiat (cons arg #f)))
            ((cdr top)
              (let ((n (string->number arg)) 
                  (msg "Invalid -t value; must be a number 0 <= n <= 250"))
                (cond 
                  ((not (number? n)) (err msg))
                  ((or (> n 250) (< n 0)) (err msg))
                  (else (set! top (cons n #f))))))
            (else 
              (let ((split (string-split arg #\:)))
                (let 
                    ((len (length split)) 
                    (msg "Invalid conversion; format is N:TOKEN or TOKEN:N"))
                  (cond 
                    ((<= len 1) (push tokens arg))
                    ((> len 2) (err msg))
                    (else 
                      (let*
                          ((convert
                            (lambda (s)
                              (let ((attempt (string->number s)))
                                (if (not attempt) s attempt))))
                          (pair 
                            (cons 
                              (convert (car split)) 
                              (convert (cadr split))))
                          (fiat-to-crypto (number? (cdr pair))))
                        (unless 
                          (equal? 
                            (number? (car pair))
                            (not fiat-to-crypto))
                          (err msg))
                        (let* 
                            ((pair 
                              (if fiat-to-crypto
                                pair
                                (cons (cdr pair) (car pair))))
                            (name (car pair)))
                          (push convs (make-conv name (cdr pair) fiat-to-crypto))
                          (unless (member name conv-names)
                            (push conv-names name))))))))))))))
  args)

(define top (car top))
(when (equal? (+ (length convs) (length tokens) top) 0)
  (display help) (exit 1))

(define fiat (car fiat))

(define do-convs (> (length convs) 0))
(define do-tokens (> (length tokens) 0))
(define do-top (> top 0))

(define token-data '())

; API calls for individual tokens return a vector of:
; (name ("{fiat}_24h_change" . change) ("{fiat}" . value))
; Bad API calls return:
; (("status" ("error_message" . "{error}") ("error_code" . 429)))

(when do-convs
  (set! token-data (api-call
    (string-append
      "https://api.coingecko.com/api/v3/simple/price?ids="
      (string-join conv-names ",")
      "&vs_currencies="
      fiat
      "&include_24hr_change=true")))
  (let ((diff (- (length conv-names) (length token-data))))
    (unless (= diff 0)
      (err (format #f "Missing ~a token~p from API response; verify that each token is spelled correctly" diff diff))))
  (let ((e (token-index token-data "status")))
    (when e
      (err (string-append "API error: " (cdadar token-data)))))
  (display "\n")
  (map
    (lambda (c)
      (let
          ((fiat (string-upcase fiat))
          (name (string-titlecase (conv-name c)))
          (value (exact->inexact (cdaddr
            (list-ref
              token-data
              (token-index token-data (conv-name c))))))
          (fmt
            (lambda (n)
              (if (< n 1)
                (let
                    ((n
                      (-
                        CONV-DIGITS
                        1
                        (min (inexact->exact (floor (log10 n))) 0))))
                  (string-append
                    "~a~a ~a -> ~,"
                    (number->string n)
                    "h ~a~%"))
                "~a~a ~a -> ~,2h ~a~%"))))
        (if (conv-ftoc c)
          (let ((conv-value (/ (conv-amount c) value)))
            (format #t
              (fmt conv-value)
              CONVS-PADDING
              (conv-amount c)
              fiat
              conv-value
              name))
          (let ((conv-value (* (conv-amount c) value)))
            (format #t
              (fmt conv-value)
              CONVS-PADDING
              (conv-amount c)
              name
              conv-value
              fiat)))))
    convs)
  (unless (or do-tokens do-top)
    (display "\n")))

(when do-tokens
  (unless (equal? tokens conv-names)
    (set! token-data (api-call
      (string-append
        "https://api.coingecko.com/api/v3/simple/price?ids="
        (string-join tokens ",")
        "&vs_currencies="
        fiat
        "&include_24hr_change=true")))
    (let ((diff (- (length tokens) (length token-data))))
      (unless (= diff 0)
        (err (format #f "Missing ~a token~p from API response; verify that each token is spelled correctly" diff diff))))
    (let ((e (token-index token-data "status")))
      (when e
        (err (string-append "API error: " (cdadar token-data))))))
  (display "\n")
  (let loop ((data token-data))
    (let ((this (car data)))
      (format #t "~a~a ~a ~a~%" 
        TOKENS-PADDING
        (format-name (car this) #f)
        (format-value (cdaddr this))
        (format-change (cdadr this))))
    (unless (equal? (cdr data) '())
      (loop (cdr data))))
  (unless do-top 
    (display "\n")))

(when do-top  
  (let 
      ((data (api-call
        (string-append 
          "https://api.coingecko.com/api/v3/coins/markets?per_page="
          (number->string top)
          "&page=1&price_change_percentage=24h&vs_currency="
          fiat))))
    (catch #t ; Only a successful call to this endpoint returns a vector
      (lambda () (value-assoc "market_cap_rank" (car (vector->list data))))
      (lambda _ (err api-fail-msg) '()))
    (display "\n")
    (let loop ((data (vector->list data)))
      (let ((this (car data)))
        (format #t "~a~a ~a ~a ~a~%"
          TOP-PADDING
          (format-rank (value-assoc "market_cap_rank" this))
          (format-name (value-assoc "name" this) #t)
          (format-value (value-assoc "current_price" this))
          (format-change (value-assoc "price_change_percentage_24h" this))))
      (unless (equal? (cdr data) '()) 
        (loop (cdr data))))
    (display "\n")))
